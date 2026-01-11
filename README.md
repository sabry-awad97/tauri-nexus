<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tauri/tauri-original.svg" width="100" alt="Tauri Logo">
</p>

<h1 align="center">⚡ Tauri Nexus</h1>

<p align="center">
  <strong>Type-safe RPC for Tauri v2 — End-to-end type safety from Rust to React</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-v2-blue?style=flat-square&logo=tauri" alt="Tauri v2">
  <img src="https://img.shields.io/badge/React-19-61dafb?style=flat-square&logo=react" alt="React 19">
  <img src="https://img.shields.io/badge/TypeScript-5.9-blue?style=flat-square&logo=typescript" alt="TypeScript">
  <img src="https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust" alt="Rust">
</p>

---

## 📦 Packages

| Package                                          | Description                              | Version                                                                       |
| ------------------------------------------------ | ---------------------------------------- | ----------------------------------------------------------------------------- |
| [`@tauri-nexus/rpc-core`](./packages/rpc-core)   | Core RPC client — framework agnostic     | ![npm](https://img.shields.io/npm/v/@tauri-nexus/rpc-core?style=flat-square)  |
| [`@tauri-nexus/rpc-react`](./packages/rpc-react) | React hooks + TanStack Query integration | ![npm](https://img.shields.io/npm/v/@tauri-nexus/rpc-react?style=flat-square) |
| [`@tauri-nexus/rpc-docs`](./packages/rpc-docs)   | Auto-generated API documentation         | ![npm](https://img.shields.io/npm/v/@tauri-nexus/rpc-docs?style=flat-square)  |

---

## ✨ Features

- 🔒 **End-to-end type safety** — Define your contract once, get full inference everywhere
- 📦 **Batch requests** — Execute multiple calls in a single IPC round-trip
- 📡 **Real-time subscriptions** — First-class streaming with async iterators
- 🔗 **TauriLink** — Composable interceptor chain (like tRPC/oRPC links)
- ✅ **Zod validation** — Optional runtime validation with schema inference
- ⚛️ **React hooks** — `useSubscription`, `useBatch`, TanStack Query integration
- 📚 **Auto-generated docs** — Interactive API explorer with live testing

---

## 🚀 Quick Start

### Installation

```bash
# Core package (framework-agnostic)
npm install @tauri-nexus/rpc-core

# React integration
npm install @tauri-nexus/rpc-react @tanstack/react-query

# API documentation (optional)
npm install @tauri-nexus/rpc-docs
```

### Define Your Contract

```typescript
// contract.ts
interface AppContract {
  health: { type: "query"; input: void; output: { status: string } };

  user: {
    get: { type: "query"; input: { id: number }; output: User };
    create: { type: "mutation"; input: CreateUserInput; output: User };
  };

  notifications: {
    stream: { type: "subscription"; input: void; output: Notification };
  };
}
```

### Create a Client

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

### Use in React

```tsx
import { useQuery, useMutation } from "@tanstack/react-query";
import { useSubscription } from "@tauri-nexus/rpc-react";
import { api, rpc } from "./contract";

function App() {
  // Queries
  const { data: user } = useQuery(
    api.user.get.queryOptions({ input: { id: 1 } })
  );

  // Mutations
  const createUser = useMutation(api.user.create.mutationOptions());

  // Subscriptions
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
│                        React Application                         │
├─────────────────────────────────────────────────────────────────┤
│  useQuery  │  useMutation  │  useSubscription  │  useBatch      │
├─────────────────────────────────────────────────────────────────┤
│                    @tauri-nexus/rpc-react                        │
│              TanStack Query Utils • React Hooks                  │
├─────────────────────────────────────────────────────────────────┤
│                    @tauri-nexus/rpc-core                         │
│     TauriLink • Interceptors • Batch • Subscriptions • Zod      │
├─────────────────────────────────────────────────────────────────┤
│                      Tauri IPC Bridge                            │
│                   invoke() • listen()                            │
├─────────────────────────────────────────────────────────────────┤
│                    tauri-plugin-rpc (Rust)                       │
│           Router • Middleware • Subscription Manager             │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📁 Project Structure

```
tauri-nexus/
├── packages/
│   ├── rpc-core/           # Core RPC client (framework-agnostic)
│   │   ├── src/
│   │   │   ├── client.ts       # Client implementation
│   │   │   ├── link.ts         # TauriLink & interceptors
│   │   │   ├── schema.ts       # Zod validation
│   │   │   ├── event-iterator.ts
│   │   │   └── types.ts
│   │   └── README.md
│   │
│   ├── rpc-react/          # React integration
│   │   ├── src/
│   │   │   ├── hooks.ts        # useSubscription, useBatch
│   │   │   └── tanstack.ts     # TanStack Query utils
│   │   └── README.md
│   │
│   └── rpc-docs/           # API documentation
│       ├── src/
│       │   ├── ApiDocs.tsx
│       │   ├── ProcedureCard.tsx
│       │   └── ...
│       └── README.md
│
├── apps/
│   └── app/                # Demo application
│       ├── src/
│       │   ├── rpc/contract.tsx
│       │   └── routes/
│       └── src-tauri/      # Rust backend
│
└── README.md
```

---

## 📖 Documentation

| Package | README                                                         |
| ------- | -------------------------------------------------------------- |
| Core    | [packages/rpc-core/README.md](./packages/rpc-core/README.md)   |
| React   | [packages/rpc-react/README.md](./packages/rpc-react/README.md) |
| Docs    | [packages/rpc-docs/README.md](./packages/rpc-docs/README.md)   |

---

## 🎮 Examples

### Batch Requests

```typescript
const response = await rpc
  .batch()
  .add("health", "health", undefined)
  .add("user1", "user.get", { id: 1 })
  .add("user2", "user.get", { id: 2 })
  .execute();

const health = response.getResult("health");
const user1 = response.getResult("user1");
```

### TauriLink with Interceptors

```typescript
import {
  TauriLink,
  createClientFromLink,
  logging,
  retry,
} from "@tauri-nexus/rpc-core";

const link = new TauriLink({
  interceptors: [
    logging({ prefix: "[RPC]" }),
    retry({ maxRetries: 3 }),
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
          name: z.string().min(1),
          email: z.string().email(),
        })
      )
      .output(z.object({ id: z.number(), name: z.string() }))
      .mutation(),
  }),
});

const rpc = createValidatedClient(contract, link);
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

# Run tests
bun run test              # All packages
bun run test --filter rpc-core   # Single package

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

## 📄 License

MIT © Tauri Nexus

---

<p align="center">
  <strong>Built with ❤️ using Tauri, React, and Rust</strong>
</p>
