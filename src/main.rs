mod config;
mod rules;
mod state;

use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, Method, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use clap::Parser;
use colored::Colorize;
use config::{MockConfig, Route};
use notify::Watcher;
use rules::RuleCondition;
use state::StateStore;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;

#[derive(Parser, Debug)]
#[command(name = "mockforge", version = "0.1.1", about = "MockForge API Gateway")]
struct Args {
    #[arg(short, long, default_value = "mockforge.yaml")]
    config: String,

    #[arg(short, long, default_value = "8080")]
    port: u16,

    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,
}

struct AppState {
    config: Arc<RwLock<MockConfig>>,
    state_store: StateStore,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config_path = PathBuf::from(&args.config);

    // Initial load
    let config = match MockConfig::load_from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                format!("Error loading config file '{}': {}", args.config, e).red()
            );
            std::process::exit(1);
        }
    };

    println!("{}", "==============================================".cyan());
    println!("{}", "   MockForge - Dynamic Multi-Tenant Gateway   ".cyan().bold());
    println!("{}", "==============================================".cyan());
    println!("Loading configuration from: {}", args.config.yellow());

    let app_state = Arc::new(AppState {
        config: Arc::new(RwLock::new(config)),
        state_store: StateStore::new(),
    });

    // Start config file watcher
    watch_config(config_path, app_state.clone());

    // Build Axum router
    let app = Router::new()
        .fallback(any(handle_request))
        .layer(CorsLayer::permissive())
        .with_state(app_state.clone());

    let addr_str = format!("{}:{}", args.host, args.port);
    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", format!("Invalid host/port: {}", e).red());
            std::process::exit(1);
        }
    };

    println!("Server listening on: {}", format!("http://{}", addr).green().bold());
    println!("{}", "Press Ctrl+C to stop.".dimmed());

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_request(
    State(app_state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(query_params): Query<HashMap<String, String>>,
    body_str: String,
) -> impl IntoResponse {
    let host_header = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let config_guard = app_state.config.read().unwrap();
    let mut matched_tenant = None;
    let mut subpath = uri.path().to_string();

    // 1. Resolve Tenant by Host
    for tenant in &config_guard.tenants {
        if let Some(ref host) = tenant.host {
            if host_header == host || host_header.starts_with(&format!("{}:", host)) {
                matched_tenant = Some(tenant.clone());
                break;
            }
        }
    }

    // 2. Resolve Tenant by Prefix
    if matched_tenant.is_none() {
        for tenant in &config_guard.tenants {
            if let Some(ref prefix) = tenant.prefix {
                if uri.path().starts_with(prefix) {
                    matched_tenant = Some(tenant.clone());
                    let stripped = &uri.path()[prefix.len()..];
                    subpath = if stripped.is_empty() { "/" } else { stripped }.to_string();
                    break;
                }
            }
        }
    }

    // 3. Fallback Tenant (if any tenant doesn't specify host or prefix, treat it as general fallback)
    if matched_tenant.is_none() {
        for tenant in &config_guard.tenants {
            if tenant.prefix.is_none() && tenant.host.is_none() {
                matched_tenant = Some(tenant.clone());
                break;
            }
        }
    }

    let tenant = match matched_tenant {
        Some(t) => t,
        None => {
            println!("{} Route not found: {}", "[404]".red(), uri.path().red());
            return build_response(
                404,
                Some(serde_json::json!({ "error": "Tenant not resolved" })),
                None,
            );
        }
    };

    // 4. Match Route inside Tenant
    let mut matched_route: Option<(&Route, HashMap<String, String>)> = None;
    for route in &tenant.routes {
        if route.method.to_uppercase() == method.as_str().to_uppercase() {
            if let Some(params) = match_path_pattern(&route.path, &subpath) {
                matched_route = Some((route, params));
                break;
            }
        }
    }

    let (route, path_params) = match matched_route {
        Some(r) => r,
        None => {
            println!(
                "{} [{}] Route not found: {} (Tenant: {})",
                "[404]".red(),
                method.as_str().yellow(),
                subpath.red(),
                tenant.name.cyan()
            );
            return build_response(
                404,
                Some(serde_json::json!({ "error": "Route not matched for tenant" })),
                None,
            );
        }
    };

    println!(
        "{} [{}] matched: {} -> {} (Tenant: {})",
        "[Match]".green(),
        method.as_str().yellow(),
        uri.path().magenta(),
        route.path.cyan(),
        tenant.name.cyan()
    );

    // Parse request headers (lowercase keys)
    let mut req_headers = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(val_str) = v.to_str() {
            req_headers.insert(k.as_str().to_lowercase(), val_str.to_string());
        }
    }

    // Parse body as JSON if possible
    let body_json: Option<serde_json::Value> = serde_json::from_str(&body_str).ok();

    // 5. Evaluate Rules
    if let Some(ref rules) = route.rules {
        for rule in rules {
            if let Some(cond) = RuleCondition::parse(&rule.r#if) {
                if cond.evaluate(&path_params, &query_params, &req_headers, body_json.as_ref()) {
                    println!("  ↳ Rule matched: {}", rule.r#if.yellow());
                    return build_response(
                        rule.status.unwrap_or(200),
                        rule.body.clone(),
                        rule.headers.clone(),
                    );
                }
            }
        }
    }

    // 6. Stateful CRUD Store Logic
    if let Some(ref resource_name) = route.state {
        let default_body = route.default.as_ref().and_then(|d| d.body.clone());
        app_state.state_store.ensure_resource(&tenant.name, resource_name, default_body);

        let method_str = method.as_str().to_uppercase();
        if method_str == "GET" {
            if !path_params.is_empty() {
                // Get single item (find the first value in the extracted path parameters)
                if let Some(id_val) = path_params.values().next() {
                    if let Some(item) = app_state.state_store.get_by_id(&tenant.name, resource_name, id_val) {
                        return build_response(200, Some(item), None);
                    }
                }
                // Fallback to route default or 404
                if let Some(ref def) = route.default {
                    return build_response(def.status.unwrap_or(404), def.body.clone(), def.headers.clone());
                }
                return build_response(
                    404,
                    Some(serde_json::json!({ "error": "Item not found in state" })),
                    None,
                );
            } else {
                // Get collection
                let list = app_state.state_store.get_all(&tenant.name, resource_name);
                return build_response(200, Some(serde_json::Value::Array(list)), None);
            }
        } else if method_str == "POST" {
            if let Some(val) = body_json {
                let inserted = app_state.state_store.insert(&tenant.name, resource_name, val);
                return build_response(201, Some(inserted), None);
            } else {
                return build_response(
                    400,
                    Some(serde_json::json!({ "error": "Invalid JSON payload for POST" })),
                    None,
                );
            }
        } else if method_str == "PUT" || method_str == "PATCH" {
            if let Some(id_val) = path_params.values().next() {
                if let Some(val) = body_json {
                    if let Some(updated) = app_state.state_store.update(&tenant.name, resource_name, id_val, val) {
                        return build_response(200, Some(updated), None);
                    }
                    return build_response(
                        404,
                        Some(serde_json::json!({ "error": "Item to update not found in state" })),
                        None,
                    );
                }
            }
            return build_response(
                400,
                Some(serde_json::json!({ "error": "Missing ID param or invalid JSON body" })),
                None,
            );
        } else if method_str == "DELETE" {
            if let Some(id_val) = path_params.values().next() {
                if app_state.state_store.delete(&tenant.name, resource_name, id_val) {
                    return build_response(204, None, None);
                }
                return build_response(
                    404,
                    Some(serde_json::json!({ "error": "Item to delete not found in state" })),
                    None,
                );
            }
            return build_response(
                400,
                Some(serde_json::json!({ "error": "Missing ID param for DELETE" })),
                None,
            );
        }
    }

    // 7. Route Default Response (fallback)
    if let Some(ref def) = route.default {
        build_response(
            def.status.unwrap_or(200),
            def.body.clone(),
            def.headers.clone(),
        )
    } else {
        build_response(200, None, None)
    }
}

fn build_response(
    status_code: u16,
    resp_body: Option<serde_json::Value>,
    resp_headers: Option<HashMap<String, String>>,
) -> Response {
    let mut builder = Response::builder().status(status_code);

    let mut has_content_type = false;
    if let Some(headers) = resp_headers {
        for (k, v) in headers {
            if k.to_lowercase() == "content-type" {
                has_content_type = true;
            }
            builder = builder.header(k, v);
        }
    }

    if !has_content_type && resp_body.is_some() {
        builder = builder.header("content-type", "application/json");
    }

    let body_bytes = if let Some(body_val) = resp_body {
        serde_json::to_vec(&body_val).unwrap_or_default()
    } else {
        Vec::new()
    };

    builder.body(Body::from(body_bytes)).unwrap()
}

fn match_path_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_parts.len() != path_parts.len() {
        return None;
    }

    let mut params = HashMap::new();
    for (pat_part, path_part) in pattern_parts.into_iter().zip(path_parts) {
        if pat_part.starts_with(':') {
            let param_name = &pat_part[1..];
            params.insert(param_name.to_string(), path_part.to_string());
        } else if pat_part != path_part {
            return None;
        }
    }
    Some(params)
}

fn watch_config(path: PathBuf, app_state: Arc<AppState>) {
    let path_clone = path.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = match notify::RecommendedWatcher::new(tx, notify::Config::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to initialize file watcher: {}", e);
            return;
        }
    };

    if let Err(e) = watcher.watch(&path, notify::RecursiveMode::NonRecursive) {
        eprintln!("Failed to watch config file: {}", e);
        return;
    }

    std::thread::spawn(move || {
        let _watcher = watcher;
        for res in rx {
            match res {
                Ok(notify::Event { kind, .. }) if kind.is_modify() => {
                    // Slight sleep to let the file complete writing
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    match MockConfig::load_from_file(&path_clone) {
                        Ok(new_config) => {
                            let mut config_guard = app_state.config.write().unwrap();
                            *config_guard = new_config;
                            println!(
                                "{}",
                                "[MockForge] Hot-reloaded configuration successfully!".green().bold()
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "{}",
                                format!("[MockForge] Failed to reload config: {}", e).red()
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    });
}
