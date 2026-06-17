use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct StateStore {
    // tenant -> resource_name -> list of values
    pub data: Arc<RwLock<HashMap<String, HashMap<String, Vec<Value>>>>>,
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initializes a resource list with default data if it doesn't already exist.
    pub fn ensure_resource(&self, tenant: &str, resource: &str, default_data: Option<Value>) {
        let mut data = self.data.write().unwrap();
        let tenant_store = data.entry(tenant.to_string()).or_default();
        if !tenant_store.contains_key(resource) {
            let initial_vec = match default_data {
                Some(Value::Array(arr)) => arr,
                Some(val) => vec![val],
                None => Vec::new(),
            };
            tenant_store.insert(resource.to_string(), initial_vec);
        }
    }

    pub fn get_all(&self, tenant: &str, resource: &str) -> Vec<Value> {
        let data = self.data.read().unwrap();
        if let Some(tenant_store) = data.get(tenant) {
            if let Some(list) = tenant_store.get(resource) {
                return list.clone();
            }
        }
        Vec::new()
    }

    pub fn get_by_id(&self, tenant: &str, resource: &str, id: &str) -> Option<Value> {
        let data = self.data.read().unwrap();
        let tenant_store = data.get(tenant)?;
        let list = tenant_store.get(resource)?;
        list.iter()
            .find(|item| get_item_id(item).as_deref() == Some(id))
            .cloned()
    }

    pub fn insert(&self, tenant: &str, resource: &str, mut item: Value) -> Value {
        let mut data = self.data.write().unwrap();
        let tenant_store = data.entry(tenant.to_string()).or_default();
        let list = tenant_store.entry(resource.to_string()).or_default();

        // Ensure item has an ID
        let has_id = get_item_id(&item).is_some();
        if !has_id {
            if let Value::Object(ref mut map) = item {
                // Generate a simple sequential ID
                let next_id = list.len() + 1;
                map.insert("id".to_string(), Value::String(next_id.to_string()));
            }
        }

        list.push(item.clone());
        item
    }

    pub fn update(&self, tenant: &str, resource: &str, id: &str, mut new_item: Value) -> Option<Value> {
        let mut data = self.data.write().unwrap();
        let tenant_store = data.get_mut(tenant)?;
        let list = tenant_store.get_mut(resource)?;

        if let Some(pos) = list.iter().position(|item| get_item_id(item).as_deref() == Some(id)) {
            // Retain original ID in the updated item
            if let Value::Object(ref mut map) = new_item {
                if !map.contains_key("id") {
                    map.insert("id".to_string(), Value::String(id.to_string()));
                }
            }
            list[pos] = new_item.clone();
            Some(new_item)
        } else {
            None
        }
    }

    pub fn delete(&self, tenant: &str, resource: &str, id: &str) -> bool {
        let mut data = self.data.write().unwrap();
        if let Some(tenant_store) = data.get_mut(tenant) {
            if let Some(list) = tenant_store.get_mut(resource) {
                if let Some(pos) = list.iter().position(|item| get_item_id(item).as_deref() == Some(id)) {
                    list.remove(pos);
                    return true;
                }
            }
        }
        false
    }
}

pub fn get_item_id(val: &Value) -> Option<String> {
    match val {
        Value::Object(map) => {
            if let Some(id_val) = map.get("id").or_else(|| map.get("ID")).or_else(|| map.get("_id")) {
                match id_val {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => Some(id_val.to_string()),
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
