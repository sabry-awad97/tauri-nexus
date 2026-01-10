<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tauri/tauri-original.svg" width="80" alt="Tauri Logo">
</p>

<h1 align="center">tauri-plugin-rpc</h1>

<p align="center">
  <strong>Type-safe, ORPC-style RPC framework for Tauri v2</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#api-reference">API</a> •
  <a href="#react-hooks">React Hooks</a>
</p>

---

## ✨ Features

- 🔒 **Type-safe** — Full TypeScript support with inferred types
- 🛣️ **Router-based** — ORPC-style nested routers with namespacing
- 🎯 **Context injection** — Pass services and state to handlers
- 🔗 **Middleware** — Async middleware chain for logging, auth, etc.
- ⚛️ **React hooks** — Built-in hooks for queries and mutations
- 📦 **Zero codegen** — No build step required, just mirror your types
- 🚀 **Lightweight** — Minimal dependencies, maximum performance

---

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tauri-plugin-rpc = { path = "../tauri-plugin-rpc" }
```

---

## 🚀 Quick Start

### 1. Define your context

```rust
// src/rpc/context.rs
#[derive(Clone)]
pub struct AppContext {
    pub db: DbService,
}

impl AppContext {
    pub fn new() -> Self {
        Self { db: DbService::new() }
    }
}
```

### 2. Create handlers

```rust
// src/rpc/handlers.rs
use tauri_plugin_rpc::prelude::*;

async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    ctx.db
        .get_user(input.id)
        .ok_or_else(|| RpcError::not_found("User not found"))
}

async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    // Validation
    if input.name.is_empty() {
        return Err(RpcError::validation("Name is required"));
    }

    ctx.db.create_user(&input.name, &input.email)
        .ok_or_else(|| RpcError::internal("Failed to create user"))
}
```

### 3. Build your router

```rust
// src/rpc/handlers.rs
pub fn create_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .middleware(logging)
        .query("health", health_handler)
        .merge("user", user_router())
}

fn user_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .query("get", get_user)
        .query("list", list_users)
        .mutation("create", create_user)
        .mutation("delete", delete_user)
}
```

### 4. Register the plugin

```rust
// src/lib.rs
use rpc::create_router;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_rpc::init(create_router()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 5. Add permissions

```json
// src-tauri/capabilities/default.json
{
  "permissions": ["core:default", "rpc:default"]
}
```

---

## 🌐 TypeScript Client

### Define your types

```typescript
// src/generated/types.ts
export interface User {
  id: number;
  name: string;
  email: string;
  createdAt: string;
}

export interface CreateUserInput {
  name: string;
  email: string;
}
```

### Create the router

```typescript
// src/generated/router.ts
import { invoke } from "@tauri-apps/api/core";

async function call<T>(path: string, input: unknown = {}): Promise<T> {
  return invoke<T>("plugin:rpc|rpc_call", { path, input });
}

export const user = {
  get: (input: { id: number }) => call<User>("user.get", input),
  list: () => call<User[]>("user.list", {}),
  create: (input: CreateUserInput) => call<User>("user.create", input),
  delete: (input: { id: number }) =>
    call<SuccessResponse>("user.delete", input),
} as const;
```

### Use it

```typescript
// Vanilla TypeScript
const users = await user.list();
const newUser = await user.create({
  name: "Alice",
  email: "alice@example.com",
});

// With error handling
try {
  const user = await user.get({ id: 1 });
} catch (error) {
  if (error.code === "NOT_FOUND") {
    console.log("User not found");
  }
}
```

---

## ⚛️ React Hooks

### Setup provider

```tsx
import { RpcProvider } from "./generated";

function App() {
  return (
    <RpcProvider>
      <MyComponent />
    </RpcProvider>
  );
}
```

### Query hooks

```tsx
function UserProfile({ id }: { id: number }) {
  const { data, isLoading, error, refetch } = useUser(id);

  if (isLoading) return <Spinner />;
  if (error) return <Error message={error.message} />;

  return (
    <div>
      <h1>{data.name}</h1>
      <button onClick={() => refetch()}>Refresh</button>
    </div>
  );
}
```

### Mutation hooks

```tsx
function CreateUserForm() {
  const { mutate, isLoading, error } = useCreateUser({
    onSuccess: (user) => {
      toast.success(`Created ${user.name}`);
    },
    onError: (error) => {
      toast.error(error.message);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        mutate({ name, email });
      }}
    >
      {/* form fields */}
      <button disabled={isLoading}>
        {isLoading ? "Creating..." : "Create User"}
      </button>
    </form>
  );
}
```

### Hook options

```tsx
// Auto-refetch every 30 seconds
const { data } = useUsers({ refetchInterval: 30000 });

// Conditional fetching
const { data } = useUser(id, { enabled: !!id });
```

---

## 🔧 Middleware

```rust
use tauri_plugin_rpc::middleware::{Request, Response, Next};

async fn logging(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    let start = std::time::Instant::now();
    println!("→ [{}] {}", req.procedure_type, req.path);

    let result = next(ctx, req.clone()).await;

    println!("← {} ({:?})", req.path, start.elapsed());
    result
}

async fn auth(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    // Check authentication
    if !is_authenticated(&req) {
        return Err(RpcError::unauthorized("Not authenticated"));
    }
    next(ctx, req).await
}

// Apply middleware
Router::new()
    .middleware(logging)
    .middleware(auth)
    .query("protected", protected_handler)
```

---

## 🎯 Error Handling

### Rust

```rust
// Built-in error constructors
RpcError::not_found("User not found")
RpcError::bad_request("Invalid input")
RpcError::validation("Email is required")
RpcError::unauthorized("Not authenticated")
RpcError::forbidden("Access denied")
RpcError::internal("Something went wrong")
RpcError::conflict("User already exists")

// Custom error with details
RpcError::new("CUSTOM_ERROR", "Something happened")
    .with_details(json!({ "field": "email" }))
```

### TypeScript

```typescript
import { isRpcError, hasErrorCode } from "./generated";

try {
  await user.create(input);
} catch (error) {
  if (isRpcError(error)) {
    switch (error.code) {
      case "VALIDATION_ERROR":
        showFieldError(error.details?.field);
        break;
      case "CONFLICT":
        showToast("User already exists");
        break;
      default:
        showToast(error.message);
    }
  }
}
```

---

## 📁 Project Structure

```
your-app/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              # Plugin registration
│   │   └── rpc/
│   │       ├── mod.rs          # Module exports
│   │       ├── context.rs      # App context & services
│   │       ├── handlers.rs     # Router & handlers
│   │       └── types.rs        # Rust types
│   └── capabilities/
│       └── default.json        # Permissions
│
├── src/
│   └── generated/
│       ├── index.ts            # Exports
│       ├── types.ts            # TypeScript types (mirror of Rust)
│       ├── client.ts           # RPC client
│       ├── router.ts           # Procedure definitions
│       └── hooks.tsx           # React hooks
│
└── tauri-plugin-rpc/           # The plugin
    └── src/
        ├── lib.rs              # Plugin exports
        ├── router.rs           # Router implementation
        ├── context.rs          # Context types
        ├── handler.rs          # Handler trait
        ├── middleware.rs       # Middleware types
        ├── error.rs            # Error types
        ├── plugin.rs           # Tauri integration
        └── types.rs            # Common types
```

---

## 📝 API Reference

### Router

| Method                      | Description                  |
| --------------------------- | ---------------------------- |
| `Router::new()`             | Create a new router          |
| `.context(ctx)`             | Set the context for handlers |
| `.middleware(fn)`           | Add middleware               |
| `.query(name, handler)`     | Add a read-only procedure    |
| `.mutation(name, handler)`  | Add a write procedure        |
| `.merge(namespace, router)` | Merge another router         |

### RpcError

| Constructor         | Code               |
| ------------------- | ------------------ |
| `not_found(msg)`    | `NOT_FOUND`        |
| `bad_request(msg)`  | `BAD_REQUEST`      |
| `validation(msg)`   | `VALIDATION_ERROR` |
| `unauthorized(msg)` | `UNAUTHORIZED`     |
| `forbidden(msg)`    | `FORBIDDEN`        |
| `internal(msg)`     | `INTERNAL_ERROR`   |
| `conflict(msg)`     | `CONFLICT`         |

### React Hooks

| Hook          | Type     | Description              |
| ------------- | -------- | ------------------------ |
| `useQuery`    | Query    | Fetch data with caching  |
| `useMutation` | Mutation | Perform write operations |

---

## 📄 License

MIT © 2026

---

<p align="center">
  Built with ❤️ for the Tauri ecosystem
</p>
