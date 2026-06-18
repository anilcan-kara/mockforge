# MockForge ⚡

<p align="center">
  <img src="assets/banner.png" alt="MockForge Banner" width="100%" />
</p>

<p align="center">
  <img src="assets/logo.png" alt="MockForge Logo" width="150" />
</p>

<h3 align="center">The Ultimate Multi-Tenant Mock API Gateway</h3>

<p align="center">
  <strong>High-performance, ultra-low memory, hot-reloading mock server that simulates entire SaaS backends and CRUD databases in RAM.</strong>
</p>

---

## 🚀 The Pain Point

Modern microservice or multi-tenant SaaS environments are painful to mock locally. Setting up 15-20 backend services for integration tests or frontend development is slow and hogs system resources. Postman's cloud mock servers are sluggish and require paid plans for high volume, while other mocking gateways consume hundreds of megabytes of RAM.

**MockForge** solves this with a statically compiled Rust binary. It routes traffic for multiple tenants (services) using a single YAML configuration file, executes complex matching conditions, and manages in-memory CRUD state—all consuming less than **10 MB of RAM** with **sub-millisecond response times**.

---

## ✨ Features

- 🌐 **Multi-Tenancy**: Route mock API requests by **Hostname** (e.g. `payment.local`) or **Path Prefix** (e.g. `/auth`, `/payment`).
- 💾 **In-Memory Stateful CRUD**: Bind mock endpoints to a dynamic state store. `POST`, `PUT`, `PATCH`, and `DELETE` requests directly update state in RAM, allowing subsequent `GET` requests to return the updated database state.
- ⚙️ **Rules Engine**: Serve dynamic responses using simple conditional expressions (e.g. `params.id == '404'` or `headers.x-api-key != 'secret'`).
- 🔄 **Hot Reloading**: Instantly watches your config file. Changing your mock configurations updates the server routes in real-time without restarts.
- 🏎️ **Ultra Lightweight**: Zero-dependency binary compiled in Rust. No Node.js runtime, no Docker, no external database required.

---

## 📦 Installation & Usage

### 1. Run Instantly (via npx)
No setup required. The wrapper CLI will detect your OS and download the optimized binary automatically:
```bash
npx mockforge-gateway --config mockforge.yaml --port 8080
```

### 2. Global Install
```bash
npm install -g mockforge-gateway
mockforge --config mockforge.yaml
```

### 3. Build From Source (Rust Cargo)
```bash
cargo install --git https://github.com/anilcan-kara/mockforge
mockforge --config mockforge.yaml
```

### 4. Direct Binary Download
You can download the precompiled static binary for your platform directly from the GitHub Release assets:
- 💻 **Windows (x64)**: [mockforge-win32-x64.exe](https://github.com/anilcan-kara/mockforge/releases/download/v0.1.2/mockforge-win32-x64.exe)
- 🐧 **Linux (x64)**: [mockforge-linux-x64](https://github.com/anilcan-kara/mockforge/releases/download/v0.1.2/mockforge-linux-x64)
- 🐧 **Linux (ARM64)**: [mockforge-linux-arm64](https://github.com/anilcan-kara/mockforge/releases/download/v0.1.2/mockforge-linux-arm64)
- 🍎 **macOS (x64)**: [mockforge-darwin-x64](https://github.com/anilcan-kara/mockforge/releases/download/v0.1.2/mockforge-darwin-x64)
- 🍎 **macOS (ARM64)**: [mockforge-darwin-arm64](https://github.com/anilcan-kara/mockforge/releases/download/v0.1.2/mockforge-darwin-arm64)

---

## 🛠️ Configuration Guide (`mockforge.yaml`)

Define all your mock microservices in a single, simple configuration file:

```yaml
tenants:
  - name: auth-service
    prefix: /auth
    routes:
      # GET collection - binds to state store "users"
      - path: /users
        method: GET
        state: users
        default:
          status: 200
          body:
            - id: "1"
              name: "Anilcan Kara"
              role: "admin"
            - id: "2"
              name: "Nozich Bot"
              role: "assistant"

      # GET single item - matches id param
      - path: /users/:id
        method: GET
        state: users
        rules:
          - if: "params.id == '404'"
            status: 404
            body: { "error": "User not found" }
        default:
          status: 404
          body: { "error": "User not found in state store" }

      # POST item - inserts JSON payload into "users" list
      - path: /users
        method: POST
        state: users

      # PUT item - updates the object by ID
      - path: /users/:id
        method: PUT
        state: users

      # DELETE item - removes from state
      - path: /users/:id
        method: DELETE
        state: users

      # Conditional Headers & Response matching
      - path: /status
        method: GET
        rules:
          - if: "headers.x-api-key != 'secret'"
            status: 401
            body: { "error": "Unauthorized" }
        default:
          status: 200
          body: { "status": "healthy" }

  - name: payment-service
    host: payment.local
    routes:
      - path: /charge
        method: POST
        default:
          status: 200
          body: { "chargeId": "ch_12345", "status": "succeeded" }
```

---

## 🛡️ Architecture & Request Life Cycle

```
                      [ Client Request ]
                              │
                              ▼
                      [ Resolves Tenant ]
                     ├── Host Match: host.local
                     └── Prefix Match: /prefix
                              │
                              ▼
                      [ Matches Route ]
                     (Method + Path Pattern)
                              │
                              ▼
                     [ Evaluates Rules ]
             (Evaluates conditions against request)
                ├── Match? ──► Return custom response
                └── No Match
                              │
                              ▼
                     [ Stateful CRUD? ]
                ├── Yes ──► Read/Write state store (RAM)
                └── No  ──► Return route default
```

---

## 🤝 Verification & Manual Testing

1. **Start MockForge**:
   ```bash
   npx mockforge-gateway --config mockforge.yaml
   ```
2. **Fetch all users** (GET collection):
   ```bash
   curl http://localhost:8080/auth/users
   ```
3. **Insert a new user** (POST):
   ```bash
   curl -X POST -H "Content-Type: application/json" -d '{"name": "Alice"}' http://localhost:8080/auth/users
   ```
4. **Fetch all users again** to verify the state was updated:
   ```bash
   curl http://localhost:8080/auth/users
   ```
5. **Verify path param matching and rules**:
   ```bash
   curl http://localhost:8080/auth/users/404
   ```

---

## 📜 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
