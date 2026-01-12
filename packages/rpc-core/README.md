# @tauri-nexus/rpc-core

> Production-ready, type-safe RPC client for Tauri v2 applications. Framework-agnostic core with full TypeScript inference.

[![npm version](https://img.shields.io/npm/v/@tauri-nexus/rpc-core.svg)](https://www.npmjs.com/package/@tauri-nexus/rpc-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Features

- 🔒 **End-to-end type safety** — Define your contract once, get full inference everywhere
- 🚀 **Zero runtime overhead** — Types are compile-time only
- 📦 **Type-safe batch requests** — Execute multiple calls with preserved types
- 🔄 **Real-time subscriptions** — First-class streaming with async iterators
- 🔗 **TauriLink** — Composable interceptor chain (like tRPC/oRPC links)
- ✅ **Zod validation** — Optional runtime validation with schema inference
- 🛠️ **Framework agnostic** — Works with React, Vue, Svelte, or vanilla JS
- 🔁 **Auto-retry & deduplication** — Built-in resilience utilities

## Installation

```bash
npm install @tauri-nexus/rpc-core
```

## Quick Start

### 1. Define Your Contract

```typescript
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

### 2. Create a Client

```typescript
import { createClientWithSubscriptions } from "@tauri-nexus/rpc-core";

const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: ["notifications.stream"],
});
```

### 3. Make Type-Safe Calls

```typescript
const user = await rpc.user.get({ id: 1 });
const newUser = await rpc.user.create({
  name: "Alice",
  email: "alice@example.com",
});

for await (const notification of await rpc.notifications.stream()) {
  console.log(notification);
}
```

## TauriLink & Interceptors

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
    onError((error, ctx) => analytics.track("rpc_error", { path: ctx.path })),
  ],
});

const rpc = createClientFromLink<AppContract>(link);
```

## Batch Requests

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

## Zod Validation

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
      .input(z.object({ name: z.string().min(1), email: z.string().email() }))
      .output(z.object({ id: z.number(), name: z.string() }))
      .mutation(),
  }),
});

const rpc = createValidatedClient(contract, link, { validateInput: true });
```

## Error Handling

```typescript
import { isRpcError, hasErrorCode } from "@tauri-nexus/rpc-core";

try {
  await rpc.user.get({ id: 999 });
} catch (error) {
  if (isRpcError(error) && hasErrorCode(error, "NOT_FOUND")) {
    console.log("User not found");
  }
}
```

## Error Codes

| Code               | Description             |
| ------------------ | ----------------------- |
| `BAD_REQUEST`      | Malformed request       |
| `UNAUTHORIZED`     | Authentication required |
| `FORBIDDEN`        | Access denied           |
| `NOT_FOUND`        | Resource not found      |
| `VALIDATION_ERROR` | Input validation failed |
| `RATE_LIMITED`     | Too many requests       |
| `INTERNAL_ERROR`   | Server error            |

## TypeScript Utilities

```typescript
import type {
  InferInput,
  InferOutput,
  ExtractPaths,
} from "@tauri-nexus/rpc-core";

type UserInput = InferInput<AppContract["user"]["get"]>; // { id: number }
type AllPaths = ExtractPaths<AppContract>; // "health" | "user.get" | ...
```

## Related Packages

- [`@tauri-nexus/rpc-react`](../rpc-react) — React hooks and TanStack Query
- [`@tauri-nexus/rpc-docs`](../rpc-docs) — API documentation UI

## License

MIT © Tauri Nexus
