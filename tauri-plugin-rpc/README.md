<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tauri/tauri-original.svg" width="100" alt="Tauri Logo">
</p>

<h1 align="center">🚀 tauri-plugin-rpc</h1>

<p align="center">
  <strong>A production-ready, type-safe RPC framework for Tauri v2 applications</strong>
</p>

<p align="center">
  <a href="#-features">Features</a> •
  <a href="#-architecture">Architecture</a> •
  <a href="#-installation">Installation</a> •
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-router-api">Router API</a> •
  <a href="#-subscriptions">Subscriptions</a> •
  <a href="#-middleware">Middleware</a> •
  <a href="#-error-handling">Error Handling</a> •
  <a href="#-typescript-client">TypeScript Client</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-v2-blue?style=flat-square&logo=tauri" alt="Tauri v2">
  <img src="https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/TypeScript-5.0+-blue?style=flat-square&logo=typescript" alt="TypeScript">
  <img src="https://img.shields.io/badge/License-MIT-green?style=flat-square" alt="MIT License">
</p>

---

## ✨ Features

| Feature                  | Description                                              |
| ------------------------ | -------------------------------------------------------- |
| 🔒 **Type-Safe**         | End-to-end type safety from Rust to TypeScript           |
| �️ **Router-Based**       | ORPC-style nested routers with namespacing               |
| 🎯 **Context Injection** | Dependency injection for services and state              |
| 🔗 **Middleware**        | Async middleware chain with onion-model execution        |
| 📡 **Subscriptions**     | Real-time streaming with backpressure handling           |
| ⚛️ **React Hooks**       | Built-in hooks for queries, mutations, and subscriptions |
| ⚡ **Compiled Routers**  | Pre-computed middleware chains for O(1) execution        |
| �️ **Error Handling**     | Structured error codes with detailed messages            |
| 📦 **Zero Codegen**      | No build step required - just mirror your types          |

---

## 🏗️ Architecture

### System Overview

```mermaid
flowchart TB
    subgraph Frontend["Frontend (TypeScript)"]
        RC[RPC Client]
        RH[React Hooks]
        EI[Event Iterator]
    end

    subgraph Tauri["Tauri Bridge"]
        IC[Invoke Command]
        ES[Event System]
    end

    subgraph Backend["Backend (Rust)"]
        PL[Plugin Layer]
        RT[Router]
        MW[Middleware Stack]
        HD[Handlers]
        SM[Subscription Manager]
    end

    RC --> IC
    RH --> RC
    EI --> ES

    IC --> PL
    ES --> SM

    PL --> RT
    RT --> MW
    MW --> HD
    HD --> SM

    style Frontend fill:#e1f5fe
    style Tauri fill:#fff3e0
    style Backend fill:#e8f5e9
```

### Request Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant P as Plugin
    participant R as Router
    participant M as Middleware
    participant H as Handler

    C->>P: invoke("plugin:rpc|rpc_call")
    P->>P: Validate Input
    P->>R: Route Request
    R->>M: Execute Chain

    loop Middleware Stack
        M->>M: Pre-process
        M->>H: Next()
        H-->>M: Response
        M->>M: Post-process
    end

    M-->>R: Final Response
    R-->>P: Result
    P-->>C: JSON Response
```

### Subscription Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant P as Plugin
    participant SM as Subscription Manager
    participant H as Handler
    participant ES as Event System

    C->>P: rpc_subscribe(path, input)
    P->>SM: Create Subscription
    SM->>H: Start Handler
    P-->>C: subscription_id

    loop Stream Events
        H->>ES: emit(event)
        ES-->>C: Event Data
    end

    alt Client Cancels
        C->>P: rpc_unsubscribe(id)
        P->>SM: Cancel
        SM->>H: Signal Cancel
    else Stream Completes
        H->>ES: emit(completed)
        ES-->>C: Completed
    end
```

---

## 📦 Installation

### Cargo.toml

```toml
[dependencies]
tauri-plugin-rpc = { path = "../tauri-plugin-rpc" }
```

### Capabilities (src-tauri/capabilities/default.json)

```json
{
  "permissions": ["core:default", "rpc:default"]
}
```

---

## 🚀 Quick Start

### Step 1: Define Your Context

The context holds your application services and is injected into every handler.

```rust
// src-tauri/src/rpc/context.rs
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppContext {
    pub db: Arc<RwLock<Database>>,
    pub config: AppConfig,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            db: Arc::new(RwLock::new(Database::new())),
            config: AppConfig::default(),
        }
    }
}
```

### Step 2: Create Handlers

Handlers are async functions that receive context and input, returning a result.

```rust
// src-tauri/src/rpc/handlers.rs
use tauri_plugin_rpc::prelude::*;

// Query - Read-only operation
async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    let db = ctx.db.read().await;
    db.get_user(input.id)
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

// Mutation - Write operation
async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    // Validation
    if input.name.trim().is_empty() {
        return Err(RpcError::validation("Name is required"));
    }
    if !input.email.contains('@') {
        return Err(RpcError::validation("Invalid email format"));
    }

    let mut db = ctx.db.write().await;
    db.create_user(&input.name, &input.email)
}

// Handler with no input - use NoInput type
async fn health_check(_ctx: Context<AppContext>, _: NoInput) -> RpcResult<HealthResponse> {
    Ok(HealthResponse {
        status: "healthy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
```

### Step 3: Build Your Router

```rust
pub fn create_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .middleware(logging)           // Add middleware
        .query("health", health_check) // Root-level query
        .merge("user", user_router())  // Nested router
        .merge("stream", stream_router())
}

fn user_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .query("get", get_user)        // user.get
        .query("list", list_users)     // user.list
        .mutation("create", create_user) // user.create
        .mutation("delete", delete_user) // user.delete
}
```

### Step 4: Register the Plugin

```rust
// src-tauri/src/lib.rs
mod rpc;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_rpc::init(rpc::create_router()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 🛣️ Router API

### Router Methods

```mermaid
classDiagram
    class Router~Ctx~ {
        +new() Router
        +context(ctx) Router~NewCtx~
        +middleware(fn) Router
        +query(name, handler) Router
        +mutation(name, handler) Router
        +subscription(name, handler) Router
        +merge(namespace, router) Router
        +compile() CompiledRouter
    }
```

| Method                        | Description                   | Example                                 |
| ----------------------------- | ----------------------------- | --------------------------------------- |
| `context(ctx)`                | Set the context for handlers  | `.context(AppContext::new())`           |
| `middleware(fn)`              | Add middleware to the chain   | `.middleware(logging)`                  |
| `query(name, handler)`        | Add a read-only procedure     | `.query("get", get_user)`               |
| `mutation(name, handler)`     | Add a write procedure         | `.mutation("create", create_user)`      |
| `subscription(name, handler)` | Add a streaming procedure     | `.subscription("events", event_stream)` |
| `merge(namespace, router)`    | Merge another router          | `.merge("user", user_router())`         |
| `compile()`                   | Pre-compute middleware chains | `.compile()`                            |

### Compiled Router (Performance Optimization)

For production, compile your router to pre-build middleware chains:

```rust
let router = Router::new()
    .context(AppContext::new())
    .middleware(logging)
    .middleware(auth)
    .query("health", health_check)
    .compile();  // ⚡ Pre-compute middleware chains

tauri::Builder::default()
    .plugin(tauri_plugin_rpc::init(router))
```

**Benefits:**

- O(1) middleware chain lookup
- No per-request chain construction
- Reduced memory allocations

---

## 📡 Subscriptions

### Creating a Subscription Handler

```rust
use tauri_plugin_rpc::prelude::*;
use async_stream::stream;
use std::pin::pin;
use tokio_stream::StreamExt;

async fn counter_stream(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    input: CounterInput,
) -> RpcResult<EventStream<CounterEvent>> {
    let (tx, rx) = event_channel(32);

    tokio::spawn(async move {
        let event_stream = stream! {
            let mut count = input.start;
            let mut ticker = tokio::time::interval(
                Duration::from_millis(input.interval_ms)
            );

            loop {
                ticker.tick().await;

                if count >= input.start + input.max_count {
                    break;
                }

                yield Event::with_id(
                    CounterEvent { count, timestamp: Utc::now().to_rfc3339() },
                    format!("counter-{}", count)
                );
                count += 1;
            }
        };

        let mut pinned = pin!(event_stream);
        while let Some(event) = pinned.next().await {
            if sub_ctx.is_cancelled() {
                break;
            }
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(rx)
}
```

### Subscription with No Input

Use `NoInput` for subscriptions that don't require parameters:

```rust
async fn time_stream(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    _: NoInput,  // Accepts both {} and null from frontend
) -> RpcResult<EventStream<String>> {
    // ...
}
```

### Subscription Context

```mermaid
classDiagram
    class SubscriptionContext {
        +subscription_id: SubscriptionId
        +last_event_id: Option~String~
        +is_cancelled() bool
        +cancelled() Future
        +signal() Arc~CancellationSignal~
    }
```

| Property/Method   | Description                     |
| ----------------- | ------------------------------- |
| `subscription_id` | Unique ID for this subscription |
| `last_event_id`   | Last event ID for resumption    |
| `is_cancelled()`  | Check if client disconnected    |
| `cancelled()`     | Async wait for cancellation     |

---

## 🔗 Middleware

### Middleware Execution Model

```mermaid
flowchart LR
    subgraph Middleware Chain
        direction TB
        M1[Logging] --> M2[Auth]
        M2 --> M3[Rate Limit]
        M3 --> H[Handler]
        H --> M3
        M3 --> M2
        M2 --> M1
    end

    R[Request] --> M1
    M1 --> Res[Response]
```

### Creating Middleware

```rust
use tauri_plugin_rpc::middleware::{Request, Response, Next};

async fn logging(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    let start = std::time::Instant::now();
    let path = req.path.clone();
    let proc_type = req.procedure_type.clone();

    println!("→ [{:?}] {}", proc_type, path);

    let result = next(ctx, req).await;
    let duration = start.elapsed();

    match &result {
        Ok(_) => println!("← {} ({:?})", path, duration),
        Err(e) => println!("✗ {} - {} ({:?})", path, e.code, duration),
    }

    result
}

async fn auth(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    // Skip auth for public endpoints
    if req.path == "health" {
        return next(ctx, req).await;
    }

    // Check authentication
    let token = req.input.get("token")
        .and_then(|v| v.as_str());

    match token {
        Some(t) if is_valid_token(t) => next(ctx, req).await,
        _ => Err(RpcError::unauthorized("Invalid or missing token")),
    }
}
```

### Applying Middleware

```rust
Router::new()
    .middleware(logging)    // Executes first (outermost)
    .middleware(auth)       // Executes second
    .middleware(rate_limit) // Executes third (innermost)
    .query("protected", protected_handler)
```

---

## 🛡️ Error Handling

### Error Codes

```mermaid
graph TD
    subgraph Client Errors
        BAD_REQUEST
        UNAUTHORIZED
        FORBIDDEN
        NOT_FOUND
        VALIDATION_ERROR
        CONFLICT
        PAYLOAD_TOO_LARGE
    end

    subgraph Server Errors
        INTERNAL_ERROR
        NOT_IMPLEMENTED
        SERVICE_UNAVAILABLE
    end

    subgraph RPC Errors
        PROCEDURE_NOT_FOUND
        SUBSCRIPTION_ERROR
        MIDDLEWARE_ERROR
        SERIALIZATION_ERROR
    end
```

### Error Constructors

```rust
// Client errors
RpcError::bad_request("Invalid request format")
RpcError::unauthorized("Authentication required")
RpcError::forbidden("Access denied")
RpcError::not_found("Resource not found")
RpcError::validation("Email is required")
RpcError::conflict("User already exists")
RpcError::payload_too_large("Request body too large")

// Server errors
RpcError::internal("Something went wrong")
RpcError::not_implemented("Feature coming soon")
RpcError::service_unavailable("Service temporarily unavailable")

// With details
RpcError::validation("Invalid input")
    .with_details(json!({
        "field": "email",
        "reason": "must be a valid email address"
    }))
```

### Error Structure

```rust
pub struct RpcError {
    pub code: RpcErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub cause: Option<String>,
}
```

---

## 🌐 TypeScript Client

### Client Architecture

```mermaid
flowchart TB
    subgraph Client Library
        CC[createClient]
        CF[call/subscribe]
        EI[EventIterator]
        MW[Middleware]
    end

    subgraph React Integration
        UQ[useQuery]
        UM[useMutation]
        US[useSubscription]
    end

    subgraph Tauri Bridge
        INV[invoke]
        LST[listen]
    end

    CC --> CF
    CF --> MW
    MW --> INV
    EI --> LST

    UQ --> CF
    UM --> CF
    US --> EI
```

### Creating a Typed Client

```typescript
// Define your contract (mirrors Rust types)
interface RpcContract {
  health: { type: "query"; input: void; output: HealthResponse };
  user: {
    get: { type: "query"; input: { id: number }; output: User };
    list: { type: "query"; input: void; output: User[] };
    create: { type: "mutation"; input: CreateUserInput; output: User };
    delete: {
      type: "mutation";
      input: { id: number };
      output: SuccessResponse;
    };
  };
  stream: {
    counter: {
      type: "subscription";
      input: CounterInput;
      output: CounterEvent;
    };
    time: { type: "subscription"; input: void; output: string };
  };
}

// Create the client
const rpc = createClient<RpcContract>({
  subscriptionPaths: ["stream.counter", "stream.time"],
});

// Use with full type safety!
const health = await rpc.health();
const user = await rpc.user.get({ id: 1 });
const users = await rpc.user.list();
```

### Subscriptions

```typescript
// Async iterator pattern
const stream = await rpc.stream.counter({
  start: 0,
  maxCount: 100,
  intervalMs: 500,
});

for await (const event of stream) {
  console.log(`Count: ${event.count}`);
}

// With cleanup
const stream = await subscribe<CounterEvent>("stream.counter", input);

try {
  for await (const event of stream) {
    if (shouldStop) break;
    handleEvent(event);
  }
} finally {
  await stream.return(); // Cleanup
}
```

### React Hooks

```tsx
// Query hook
function UserProfile({ id }: { id: number }) {
  const { data, isLoading, error, refetch } = useQuery(
    () => rpc.user.get({ id }),
    [id]
  );

  if (isLoading) return <Spinner />;
  if (error) return <Error message={error.message} />;

  return (
    <div>
      <h1>{data.name}</h1>
      <button onClick={refetch}>Refresh</button>
    </div>
  );
}

// Mutation hook
function CreateUserForm() {
  const { mutate, isLoading, error } = useMutation(rpc.user.create, {
    onSuccess: (user) => toast.success(`Created ${user.name}`),
    onError: (error) => toast.error(error.message),
  });

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        mutate({ name, email });
      }}
    >
      {/* form fields */}
    </form>
  );
}

// Subscription hook
function CounterDisplay() {
  const { data, isConnected, error } = useSubscription<CounterEvent>(
    () => subscribe("stream.counter", { start: 0, maxCount: 100 }),
    [],
    { onEvent: (event) => console.log(event) }
  );

  return (
    <div>
      <span className={isConnected ? "connected" : "disconnected"} />
      <span>{data?.count ?? "—"}</span>
    </div>
  );
}
```

---

## ⚙️ Configuration

### RpcConfig Options

```rust
use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy};

let config = RpcConfig::new()
    // Input validation
    .with_max_input_size(1024 * 1024)  // 1MB max input

    // Subscription settings
    .with_channel_buffer(64)           // Event buffer size
    .with_backpressure_strategy(BackpressureStrategy::DropOldest)

    // Debugging
    .with_debug_logging(true);

tauri::Builder::default()
    .plugin(tauri_plugin_rpc::init_with_config(router, config))
```

### Backpressure Strategies

| Strategy     | Description                                   |
| ------------ | --------------------------------------------- |
| `Block`      | Block sender until buffer has space (default) |
| `DropOldest` | Drop oldest events when buffer is full        |
| `DropNewest` | Drop new events when buffer is full           |

---

## 📁 Project Structure

```
your-app/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                 # Plugin registration
│   │   └── rpc/
│   │       ├── mod.rs             # Module exports
│   │       ├── context.rs         # App context & services
│   │       ├── handlers.rs        # Router & handlers
│   │       └── types.rs           # Rust types
│   └── capabilities/
│       └── default.json           # Permissions
│
├── src/
│   ├── lib/
│   │   └── rpc/
│   │       ├── index.ts           # Library exports
│   │       ├── types.ts           # TypeScript types
│   │       ├── client.ts          # RPC client
│   │       ├── event-iterator.ts  # Subscription handling
│   │       └── hooks.ts           # React hooks
│   └── rpc/
│       └── contract.ts            # Your contract definition
│
└── tauri-plugin-rpc/              # The plugin
    └── src/
        ├── lib.rs                 # Plugin exports
        ├── router.rs              # Router implementation
        ├── context.rs             # Context types
        ├── handler.rs             # Handler trait
        ├── middleware.rs          # Middleware types
        ├── subscription.rs        # Subscription system
        ├── error.rs               # Error types
        ├── plugin.rs              # Tauri integration
        ├── config.rs              # Configuration
        └── types.rs               # Common types (NoInput, etc.)
```

---

## 📚 API Reference

### Prelude Exports

```rust
use tauri_plugin_rpc::prelude::*;

// This imports:
// - Router, CompiledRouter
// - Context, EmptyContext
// - RpcError, RpcErrorCode, RpcResult
// - Handler, Middleware, Next, Request
// - Event, EventStream, EventSender, event_channel
// - SubscriptionContext, SubscriptionHandler
// - NoInput, SuccessResponse, PaginatedResponse
// - init, init_with_config
// - And more...
```

### Key Types

| Type                  | Description                               |
| --------------------- | ----------------------------------------- |
| `Router<Ctx>`         | Builder for defining procedures           |
| `CompiledRouter<Ctx>` | Optimized router with pre-built chains    |
| `Context<Ctx>`        | Wrapper providing access to app context   |
| `RpcResult<T>`        | Result type alias for handlers            |
| `RpcError`            | Structured error with code and message    |
| `NoInput`             | Empty input type (accepts `{}` or `null`) |
| `EventStream<T>`      | Channel receiver for subscription events  |
| `SubscriptionContext` | Context for subscription handlers         |

---

## 🧪 Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_user() {
        let ctx = Context::new(AppContext::new());
        let input = GetUserInput { id: 1 };

        let result = get_user(ctx, input).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 1);
    }

    #[tokio::test]
    async fn test_validation_error() {
        let ctx = Context::new(AppContext::new());
        let input = CreateUserInput {
            name: "".into(),
            email: "test@example.com".into()
        };

        let result = create_user(ctx, input).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, RpcErrorCode::ValidationError);
    }
}
```

---

## 📄 License

MIT © 2026

---

<p align="center">
  <strong>Built with ❤️ for the Tauri ecosystem</strong>
</p>

<p align="center">
  <a href="https://github.com/tauri-apps/tauri">Tauri</a> •
  <a href="https://www.rust-lang.org/">Rust</a> •
  <a href="https://www.typescriptlang.org/">TypeScript</a>
</p>
