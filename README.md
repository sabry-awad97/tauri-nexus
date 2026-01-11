<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tauri/tauri-original.svg" width="100" alt="Tauri Logo">
</p>

<h1 align="center">⚡ Tauri RPC Demo</h1>

<p align="center">
  <strong>A production-ready Tauri v2 application showcasing type-safe RPC communication</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-v2-blue?style=flat-square&logo=tauri" alt="Tauri v2">
  <img src="https://img.shields.io/badge/React-18-61dafb?style=flat-square&logo=react" alt="React 18">
  <img src="https://img.shields.io/badge/TypeScript-5.0-blue?style=flat-square&logo=typescript" alt="TypeScript">
  <img src="https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Vite-5-646cff?style=flat-square&logo=vite" alt="Vite">
</p>

---

## 🎯 Overview

This project demonstrates a complete, production-ready RPC system for Tauri v2 applications with:

- **Type-safe communication** between Rust backend and TypeScript frontend
- **Real-time subscriptions** with async iterators and auto-reconnect
- **React hooks** for queries, mutations, and subscriptions
- **Middleware support** for logging, authentication, and more
- **Structured error handling** with typed error codes

---

## 🏗️ Architecture

```mermaid
flowchart TB
    subgraph Frontend["Frontend (React + TypeScript)"]
        UI[React Components]
        HK[React Hooks]
        CL[RPC Client]
        EI[Event Iterator]
    end

    subgraph Bridge["Tauri Bridge"]
        INV[invoke API]
        EVT[Event System]
    end

    subgraph Backend["Backend (Rust)"]
        PLG[tauri-plugin-rpc]
        RTR[Router]
        MW[Middleware]
        HD[Handlers]
        SM[Subscription Manager]
    end

    UI --> HK
    HK --> CL
    CL --> INV
    EI --> EVT

    INV --> PLG
    EVT --> SM
    PLG --> RTR
    RTR --> MW
    MW --> HD
    HD --> SM

    style Frontend fill:#e3f2fd
    style Bridge fill:#fff3e0
    style Backend fill:#e8f5e9
```

---

## ✨ Features

| Feature                | Description                                    |
| ---------------------- | ---------------------------------------------- |
| 🔒 **Type-Safe RPC**   | End-to-end type safety from Rust to TypeScript |
| 📡 **Subscriptions**   | Real-time streaming with backpressure handling |
| ⚛️ **React Hooks**     | `useQuery`, `useMutation`, `useSubscription`   |
| 🔗 **Middleware**      | Logging, auth, rate limiting support           |
| 🛡️ **Error Handling**  | Structured errors with typed codes             |
| ⚡ **Compiled Router** | Pre-built middleware chains for O(1) execution |
| 🔄 **Auto-Reconnect**  | Resilient subscription connections             |

---

## 📁 Project Structure

```
tauri-rpc-demo/
├── src/                          # Frontend (React + TypeScript)
│   ├── lib/
│   │   └── rpc/                  # RPC Client Library
│   │       ├── client.ts         # Client implementation
│   │       ├── hooks.ts          # React hooks
│   │       ├── event-iterator.ts # Subscription handling
│   │       └── types.ts          # Type definitions
│   ├── rpc/
│   │   └── contract.ts           # Contract definition
│   ├── routes/                   # TanStack Router pages
│   │   ├── streams/              # Subscription demos
│   │   │   ├── counter.tsx       # Counter stream
│   │   │   ├── stocks.tsx        # Stock prices
│   │   │   ├── chat.tsx          # Chat room
│   │   │   └── time.tsx          # Server time
│   │   └── ...
│   └── components/               # React components
│
├── src-tauri/                    # Backend (Rust)
│   ├── src/
│   │   ├── lib.rs                # Tauri setup
│   │   └── rpc/
│   │       ├── mod.rs            # Module exports
│   │       ├── context.rs        # App context
│   │       ├── handlers.rs       # RPC handlers
│   │       └── types.rs          # Rust types
│   └── capabilities/
│       └── default.json          # Permissions
│
└── tauri-plugin-rpc/             # RPC Plugin
    └── src/
        ├── lib.rs                # Plugin exports
        ├── router.rs             # Router implementation
        ├── subscription.rs       # Subscription system
        ├── middleware.rs         # Middleware types
        └── error.rs              # Error handling
```

---

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- [Bun](https://bun.sh/) (recommended) or npm/yarn

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd tauri-rpc-demo

# Install frontend dependencies
bun install

# Run in development mode
bun tauri dev
```

### Build for Production

```bash
bun tauri build
```

---

## 📖 Documentation

| Document                                                   | Description                     |
| ---------------------------------------------------------- | ------------------------------- |
| [tauri-plugin-rpc/README.md](./tauri-plugin-rpc/README.md) | Rust plugin documentation       |
| [src/lib/rpc/README.md](./src/lib/rpc/README.md)           | TypeScript client documentation |

---

## 🎮 Demo Features

### Queries & Mutations

```typescript
// Type-safe queries
const user = await rpc.user.get({ id: 1 });
const users = await rpc.user.list();

// Type-safe mutations
const newUser = await rpc.user.create({
  name: "Alice",
  email: "alice@example.com",
});
```

### Subscriptions

```typescript
// Counter stream
const stream = await rpc.stream.counter({
  start: 0,
  maxCount: 100,
  intervalMs: 500,
});

for await (const event of stream) {
  console.log(`Count: ${event.count}`);
}
```

### React Hooks

```tsx
// Query hook
const { data, isLoading, error } = useQuery(() => rpc.user.get({ id }), [id]);

// Mutation hook
const { mutate, isLoading } = useMutation(rpc.user.create);

// Subscription hook
const { data, isConnected } = useSubscription(
  () => subscribe("stream.counter", input),
  [],
);
```

---

## 🛠️ Development

### IDE Setup

- [VS Code](https://code.visualstudio.com/)
- [Tauri Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### Commands

```bash
# Development
bun tauri dev

# Build
bun tauri build

# Run tests
bun test

# Generate Rust docs
cd tauri-plugin-rpc && cargo doc --open
```

---

## 📊 Request Flow

```mermaid
sequenceDiagram
    participant C as React Component
    participant H as useQuery Hook
    participant CL as RPC Client
    participant T as Tauri invoke
    participant P as Plugin
    participant R as Router
    participant M as Middleware
    participant HD as Handler

    C->>H: render
    H->>CL: rpc.user.get({ id })
    CL->>T: invoke("plugin:rpc|rpc_call")
    T->>P: rpc_call(path, input)
    P->>R: route(path)
    R->>M: execute chain
    M->>HD: handler(ctx, input)
    HD-->>M: Result<User>
    M-->>R: Response
    R-->>P: JSON
    P-->>T: Result
    T-->>CL: User
    CL-->>H: data
    H-->>C: { data, isLoading: false }
```

---

## 📡 Subscription Flow

```mermaid
sequenceDiagram
    participant C as Component
    participant H as useSubscription
    participant EI as EventIterator
    participant T as Tauri Events
    participant SM as SubscriptionManager
    participant HD as Handler

    C->>H: mount
    H->>EI: subscribe(path, input)
    EI->>T: listen(event_name)
    EI->>SM: rpc_subscribe
    SM->>HD: start handler
    HD-->>SM: EventStream

    loop Stream Events
        HD->>T: emit(event)
        T->>EI: event data
        EI->>H: yield event
        H->>C: update state
    end

    C->>H: unmount
    H->>EI: return()
    EI->>SM: rpc_unsubscribe
    SM->>HD: cancel signal
```

---

## 🔧 Configuration

### Tauri Permissions

```json
// src-tauri/capabilities/default.json
{
  "permissions": ["core:default", "rpc:default"]
}
```

### RPC Configuration

```rust
use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy};

let config = RpcConfig::new()
    .with_max_input_size(1024 * 1024)
    .with_channel_buffer(64)
    .with_backpressure_strategy(BackpressureStrategy::DropOldest);
```

---

## 📄 License

MIT © 2024-2026

---

<p align="center">
  <strong>Built with ❤️ using Tauri, React, and Rust</strong>
</p>

<p align="center">
  <a href="https://tauri.app">Tauri</a> •
  <a href="https://react.dev">React</a> •
  <a href="https://www.rust-lang.org">Rust</a> •
  <a href="https://vitejs.dev">Vite</a>
</p>
