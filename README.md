<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tauri/tauri-original.svg" width="100" alt="Tauri Logo">
</p>

<h1 align="center">⚡ Tauri Nexus</h1>

<p align="center">
  <strong>Production-Ready Type-Safe RPC for Tauri v2</strong><br>
  End-to-end type safety from Rust to React with enterprise-grade features
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-v2-blue?style=flat-square&logo=tauri" alt="Tauri v2">
  <img src="https://img.shields.io/badge/React-19-61dafb?style=flat-square&logo=react" alt="React 19">
  <img src="https://img.shields.io/badge/TypeScript-5.9-blue?style=flat-square&logo=typescript" alt="TypeScript">
  <img src="https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Tests-667+-green?style=flat-square" alt="Tests">
</p>

---

## 🎯 Why Tauri Nexus?

Tauri Nexus provides a complete RPC solution for Tauri v2 applications with **33 professional features** across 12 categories. It's designed for production use with comprehensive testing (359 Rust tests, 308 TypeScript tests).

### Key Differentiators

- **oRPC-Style Architecture** — Hierarchical router with middleware chains
- **End-to-End Type Safety** — Define once, infer everywhere
- **Real-Time Subscriptions** — Backpressure handling, auto-reconnect
- **Enterprise Security** — Rate limiting, caching, authentication
- **React Integration** — TanStack Query, custom hooks, batch operations

---

## 📦 Packages

| Package                                                            | Description                  | Docs                                     |
| ------------------------------------------------------------------ | ---------------------------- | ---------------------------------------- |
| [`tauri-plugin-rpc`](./apps/app/src-tauri/crates/tauri-plugin-rpc) | Rust backend plugin          | [Rust Docs](#rust-backend)               |
| [`@tauri-nexus/rpc-core`](./packages/rpc-core)                     | Core TypeScript client       | [README](./packages/rpc-core/README.md)  |
| [`@tauri-nexus/rpc-react`](./packages/rpc-react)                   | React hooks & TanStack Query | [README](./packages/rpc-react/README.md) |
| [`@tauri-nexus/rpc-docs`](./packages/rpc-docs)                     | API documentation UI         | [README](./packages/rpc-docs/README.md)  |

---

## ✨ Feature Overview

### 🏗️ Architecture & Design

- **oRPC-Style Router** — Nested routers with path prefixes
- **Onion-Model Middleware** — Composable request/response pipeline
- **Type-Safe Context** — Dependency injection without globals

### 🔒 Security

- **Flexible Authentication** — Pluggable auth providers (JWT, session, API key)
- **Role-Based Authorization** — Fine-grained access control
- **Input Validation** — Zod schemas with runtime validation

### ⚡ Performance

- **Multi-Strategy Rate Limiting** — Fixed window, sliding window, token bucket
- **Intelligent Caching** — LRU cache with TTL and pattern invalidation
- **Compiled Routers** — Pre-computed middleware chains for O(1) lookup

### 📡 Real-Time

- **Subscription System** — Backpressure handling, event metadata
- **Auto-Reconnect** — Configurable retry with exponential backoff
- **Event Streaming** — Async iterators with cleanup

### 🎯 Type Safety

- **Contract-First Design** — Define types once, use everywhere
- **Type-Safe Batch** — Preserve types across multiple calls
- **Path Inference** — Autocomplete for procedure paths

### ⚛️ React Integration

- **useSubscription** — Real-time data with lifecycle management
- **useBatch** — Efficient multi-call operations
- **TanStack Query** — queryOptions, mutationOptions, infiniteOptions

---

## 🚀 Quick Start

### Installation

```bash
# Core package (framework-agnostic)
npm install @tauri-nexus/rpc-core

# React integration
npm install @tauri-nexus/rpc-react @tanstack/react-query
```

### 1. Define Your Contract (TypeScript)

```typescript
// contract.ts
interface AppContract {
  health: { type: "query"; input: void; output: { status: string } };

  user: {
    get: { type: "query"; input: { id: number }; output: User };
    create: { type: "mutation"; input: CreateUserInput; output: User };
    list: {
      type: "query";
      input: PaginationInput;
      output: PaginatedResponse<User>;
    };
  };

  notifications: {
    stream: { type: "subscription"; input: void; output: Notification };
  };
}
```

### 2. Create Your Router (Rust)

```rust
use tauri_plugin_rpc::prelude::*;

pub fn create_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .middleware(logging_middleware())
        .middleware(auth_middleware(TokenAuthProvider::new()))
        .query("health", health_check)
        .merge("user", user_router())
        .subscription("notifications.stream", notification_stream)
}

fn user_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .query("get", get_user)
        .query("list", list_users)
        .mutation("create", create_user)
}
```

### 3. Register the Plugin (Rust)

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_rpc::init(create_router()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 4. Create Client (TypeScript)

```typescript
import {
  createClientWithSubscriptions,
  createTanstackQueryUtils,
} from "@tauri-nexus/rpc-react";

export const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: ["notifications.stream"],
});

export const api = createTanstackQueryUtils<AppContract>(rpc);
```

### 5. Use in React

```tsx
import { useQuery, useMutation } from "@tanstack/react-query";
import { useSubscription } from "@tauri-nexus/rpc-react";

function App() {
  // Queries with full type inference
  const { data: user } = useQuery(
    api.user.get.queryOptions({ input: { id: 1 } })
  );

  // Mutations
  const createUser = useMutation(api.user.create.mutationOptions());

  // Real-time subscriptions
  const { data: notification, isConnected } = useSubscription(
    () => rpc.notifications.stream(),
    []
  );

  return <div>{user?.name}</div>;
}
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        React Application                        │
├─────────────────────────────────────────────────────────────────┤
│  useQuery  │  useMutation  │  useSubscription  │  useBatch      │
├─────────────────────────────────────────────────────────────────┤
│                    @tauri-nexus/rpc-react                       │
│              TanStack Query Utils • React Hooks                 │
├─────────────────────────────────────────────────────────────────┤
│                    @tauri-nexus/rpc-core                        │
│     TauriLink • Interceptors • Batch • Subscriptions • Zod      │
├─────────────────────────────────────────────────────────────────┤
│                      Tauri IPC Bridge                           │
│                   invoke() • listen()                           │
├─────────────────────────────────────────────────────────────────┤
│                    tauri-plugin-rpc (Rust)                      │
│    Router • Middleware • Auth • Cache • Rate Limit • Schema     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🦀 Rust Backend

### Router & Procedures

```rust
use tauri_plugin_rpc::prelude::*;

// Query - read-only operation
async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    let db = ctx.db.read().await;
    db.get_user(input.id)
        .ok_or_else(|| RpcError::not_found("User not found"))
}

// Mutation - write operation
async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    let mut db = ctx.db.write().await;
    db.create_user(&input.name, &input.email)
}

// Handler with no input
async fn health_check(_ctx: Context<AppContext>, _: NoInput) -> RpcResult<HealthResponse> {
    Ok(HealthResponse { status: "healthy".to_string() })
}
```

### Middleware

```rust
use tauri_plugin_rpc::middleware::{Request, Next};

async fn logging(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    let start = std::time::Instant::now();
    println!("→ [{:?}] {}", req.procedure_type, req.path);

    let result = next(ctx, req.clone()).await;

    match &result {
        Ok(_) => println!("← {} ({:?})", req.path, start.elapsed()),
        Err(e) => println!("✗ {} - {} ({:?})", req.path, e.code, start.elapsed()),
    }

    result
}
```

### Rate Limiting

```rust
use tauri_plugin_rpc::rate_limit::*;
use std::time::Duration;

let config = RateLimitConfig::new()
    .with_default_limit(RateLimit::sliding_window(100, Duration::from_secs(60)))
    .with_procedure_limit("expensive.operation", RateLimit::fixed_window(10, Duration::from_secs(60)))
    .with_procedure_limit("api.burst", RateLimit::token_bucket(50, Duration::from_secs(60), 10.0));

let limiter = RateLimiter::new(config);

// Use as middleware
Router::new()
    .middleware(rate_limit_middleware(limiter, |req| req.client_id()))
```

### Caching

```rust
use tauri_plugin_rpc::cache::*;
use std::time::Duration;

let config = CacheConfig::new()
    .with_default_ttl(Duration::from_secs(300))
    .with_max_entries(1000)
    .with_procedure_ttl("user.profile", Duration::from_secs(60))
    .exclude_pattern("admin.*");

let cache = Cache::new(config);

// Cache middleware for queries
Router::new()
    .middleware(cache_middleware(cache.clone()))
    // Invalidation middleware for mutations
    .middleware(invalidation_middleware(cache, vec![
        ("user.update", vec!["user.*"]),
        ("user.delete", vec!["user.*"]),
    ]))
```

### Authentication & Authorization

```rust
use tauri_plugin_rpc::auth::*;

// Custom auth provider
struct JwtAuthProvider { secret: String }

impl AuthProvider for JwtAuthProvider {
    fn authenticate(&self, request: &Request) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
        Box::pin(async move {
            if let Some(token) = extract_token(request) {
                if let Ok(claims) = validate_jwt(&token, &self.secret) {
                    return AuthResult::authenticated(claims.sub)
                        .with_roles(claims.roles);
                }
            }
            AuthResult::unauthenticated()
        })
    }
}

// Configure authorization rules
let config = AuthConfig::new()
    .public("health")
    .public("auth.login")
    .requires_auth("user.*")
    .requires_roles("admin.*", vec!["admin"]);

Router::new()
    .middleware(auth_with_config(JwtAuthProvider::new(), config))
```

### Subscriptions

```rust
use tauri_plugin_rpc::subscription::*;
use async_stream::stream;

async fn notification_stream(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    _: NoInput,
) -> RpcResult<EventStream<Notification>> {
    let (tx, rx) = event_channel(32);

    tokio::spawn(async move {
        let event_stream = stream! {
            loop {
                if sub_ctx.is_cancelled() { break; }

                // Fetch notifications...
                yield Event::with_id(
                    Notification { message: "New message".to_string() },
                    format!("notif-{}", uuid::Uuid::new_v4())
                );

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        };

        let mut pinned = std::pin::pin!(event_stream);
        while let Some(event) = pinned.next().await {
            if tx.send(event).await.is_err() { break; }
        }
    });

    Ok(rx)
}
```

### Error Handling

```rust
use tauri_plugin_rpc::{RpcError, RpcErrorCode};

// Convenience constructors
RpcError::not_found("User not found")
RpcError::bad_request("Invalid request")
RpcError::validation("Email is required")
RpcError::unauthorized("Not authenticated")
RpcError::forbidden("Access denied")
RpcError::internal("Something went wrong")
RpcError::conflict("User already exists")
RpcError::rate_limited("Too many requests")

// With additional details
RpcError::validation("Invalid input")
    .with_details(json!({ "field": "email", "reason": "invalid format" }))
    .with_cause("Regex validation failed")
```

### Configuration

```rust
use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy};

let config = RpcConfig::new()
    .with_max_input_size(1024 * 1024)  // 1MB max input
    .with_channel_buffer(64)           // Subscription buffer size
    .with_backpressure_strategy(BackpressureStrategy::DropOldest)
    .with_debug_logging(true);

tauri::Builder::default()
    .plugin(tauri_plugin_rpc::init_with_config(router, config))
```

---

## 📘 TypeScript Client

### TauriLink & Interceptors

```typescript
import {
  TauriLink,
  createClientFromLink,
  logging,
  retry,
  onError,
} from "@tauri-nexus/rpc-core";

const link = new TauriLink({
  interceptors: [
    logging({ prefix: "[RPC]" }),
    retry({ maxRetries: 3, delay: 1000 }),
    onError((error, ctx) => {
      analytics.track("rpc_error", { path: ctx.path, code: error.code });
    }),
    // Custom interceptor
    async (ctx, next) => {
      const start = Date.now();
      const result = await next();
      console.log(`${ctx.path} took ${Date.now() - start}ms`);
      return result;
    },
  ],
});

const rpc = createClientFromLink<AppContract>(link);
```

### Batch Operations

```typescript
const response = await rpc
  .batch()
  .add("health", "health", undefined)
  .add("user1", "user.get", { id: 1 })
  .add("user2", "user.get", { id: 2 })
  .execute();

// Type-safe result access
const health = response.getResult("health");
const user1 = response.getResult("user1");

console.log(response.successCount); // Number of successful calls
console.log(response.errorCount); // Number of failed calls
```

### Zod Validation

```typescript
import { z } from "zod";
import {
  procedure,
  router,
  createValidatedClient,
} from "@tauri-nexus/rpc-core";

const contract = router({
  user: router({
    create: procedure()
      .input(
        z.object({
          name: z.string().min(1).max(100),
          email: z.string().email(),
        })
      )
      .output(z.object({ id: z.number(), name: z.string(), email: z.string() }))
      .mutation(),
  }),
});

const rpc = createValidatedClient(contract, link, {
  validateInput: true,
  validateOutput: true,
});
```

### Error Handling

```typescript
import { isRpcError, hasErrorCode } from "@tauri-nexus/rpc-core";

try {
  await rpc.user.get({ id: 999 });
} catch (error) {
  if (isRpcError(error)) {
    if (hasErrorCode(error, "NOT_FOUND")) {
      console.log("User not found");
    } else if (hasErrorCode(error, "RATE_LIMITED")) {
      const retryAfter = error.details?.retry_after_ms;
      console.log(`Rate limited. Retry after ${retryAfter}ms`);
    }
  }
}
```

---

## ⚛️ React Integration

### useSubscription

```tsx
import { useSubscription } from "@tauri-nexus/rpc-react";

function NotificationFeed() {
  const { data, isConnected, error, reconnectCount } = useSubscription(
    () => rpc.notifications.stream(),
    [],
    {
      autoReconnect: true,
      maxReconnects: 5,
      reconnectDelay: 1000,
      onEvent: (notification) => console.log("New:", notification),
      onError: (error) => console.error("Error:", error),
    }
  );

  return (
    <div>
      <span>{isConnected ? "🟢" : "🔴"}</span>
      {data && <NotificationCard notification={data} />}
    </div>
  );
}
```

### useBatch

```tsx
import { useBatch } from "@tauri-nexus/rpc-react";

function Dashboard() {
  const batch = useBatch(
    () =>
      rpc
        .batch()
        .add("health", "health", undefined)
        .add("users", "user.list", { page: 1 }),
    { executeOnMount: true }
  );

  if (batch.isLoading) return <Loading />;

  const health = batch.getResult("health");
  const users = batch.getResult("users");

  return (
    <div>
      <p>Status: {health?.data?.status}</p>
      <p>Users: {users?.data?.length}</p>
      <button onClick={() => batch.execute()}>Refresh</button>
    </div>
  );
}
```

### TanStack Query

```tsx
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

function UserProfile({ userId }: { userId: number }) {
  const queryClient = useQueryClient();

  const { data: user } = useQuery(
    api.user.get.queryOptions({ input: { id: userId } })
  );

  const updateUser = useMutation({
    ...api.user.update.mutationOptions(),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: api.user.get.key({ id: userId }),
      });
    },
  });

  return <div>{user?.name}</div>;
}
```

---

## 📁 Project Structure

```
tauri-nexus/
├── packages/
│   ├── rpc-core/           # Core TypeScript client
│   ├── rpc-react/          # React hooks & TanStack Query
│   └── rpc-docs/           # API documentation UI
├── apps/
│   └── app/
│       ├── src/            # Frontend application
│       └── src-tauri/
│           └── crates/
│               └── tauri-plugin-rpc/  # Rust plugin
└── README.md
```

---

## 🛠️ Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- [Node.js](https://nodejs.org/) 18+
- [Bun](https://bun.sh/) (recommended)

### Setup

```bash
# Install dependencies
bun install

# Run all tests
bun run test

# Run Rust tests
cd apps/app/src-tauri && cargo test

# Run TypeScript tests
bun run test --filter rpc-core
bun run test --filter rpc-react

# Development
cd apps/app && bun tauri dev
```

### Build

```bash
# Build packages
bun run build

# Build Tauri app
cd apps/app && bun tauri build
```

---

## 📊 Test Coverage

| Component       | Tests    | Coverage             |
| --------------- | -------- | -------------------- |
| Rust Plugin     | 359      | Comprehensive        |
| TypeScript Core | 200+     | Full                 |
| React Hooks     | 100+     | Full                 |
| **Total**       | **667+** | **Production-Ready** |

---

## 📄 License

MIT © Tauri Nexus

---

<p align="center">
  <strong>Built with ❤️ using Tauri, React, and Rust</strong>
</p>
