# @tauri-nexus/rpc-core

> Type-safe RPC client for Tauri v2 applications. Framework-agnostic core with full TypeScript inference.

[![npm version](https://img.shields.io/npm/v/@tauri-nexus/rpc-core.svg)](https://www.npmjs.com/package/@tauri-nexus/rpc-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Features

- 🔒 **End-to-end type safety** — Define your contract once, get full inference everywhere
- 🚀 **Zero runtime overhead** — Types are compile-time only
- 📦 **Batch requests** — Execute multiple calls in a single IPC round-trip
- 🔄 **Real-time subscriptions** — First-class streaming support with async iterators
- 🔗 **TauriLink** — Composable interceptor chain (like tRPC/oRPC links)
- ✅ **Zod validation** — Optional runtime validation with schema inference
- 🛠️ **Framework agnostic** — Works with React, Vue, Svelte, or vanilla JS

## Installation

```bash
npm install @tauri-nexus/rpc-core
# or
pnpm add @tauri-nexus/rpc-core
# or
bun add @tauri-nexus/rpc-core
```

## Quick Start

### 1. Define Your Contract

```typescript
// contract.ts
interface AppContract {
  health: { type: "query"; input: void; output: { status: string } };

  user: {
    get: { type: "query"; input: { id: number }; output: User };
    create: { type: "mutation"; input: CreateUserInput; output: User };
  };

  notifications: {
    subscribe: { type: "subscription"; input: void; output: Notification };
  };
}
```

### 2. Create a Client

```typescript
import {
  createClient,
  createClientWithSubscriptions,
} from "@tauri-nexus/rpc-core";

// Basic client (queries + mutations)
const rpc = createClient<AppContract>();

// With subscription support
const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: ["notifications.subscribe"],
});
```

### 3. Make Type-Safe Calls

```typescript
// Queries - input is type-checked, output is inferred
const health = await rpc.health();
// => { status: string }

const user = await rpc.user.get({ id: 1 });
// => User

// Mutations
const newUser = await rpc.user.create({
  name: "Alice",
  email: "alice@example.com",
});
// => User

// Subscriptions (async iterator)
for await (const notification of rpc.notifications.subscribe()) {
  console.log(notification);
  // => Notification
}
```

## Core Concepts

### Contract Definition

Contracts define your RPC API structure with full type information:

```typescript
interface MyContract {
  // Simple procedure
  procedureName: {
    type: "query" | "mutation" | "subscription";
    input: InputType; // use `void` for no input
    output: OutputType;
  };

  // Nested namespace
  namespace: {
    nestedProcedure: { type: "query"; input: void; output: string };
  };
}
```

### Batch Requests

Reduce IPC overhead by batching multiple calls:

```typescript
const response = await rpc
  .batch()
  .add("health", "health", undefined)
  .add("user1", "user.get", { id: 1 })
  .add("user2", "user.get", { id: 2 })
  .execute();

// Type-safe result access
const healthResult = response.getResult("health");
if (healthResult.data) {
  console.log(healthResult.data.status);
}

// Batch utilities
console.log(response.successCount); // number of successful calls
console.log(response.errorCount); // number of failed calls
response.getSuccessful(); // all successful results
response.getFailed(); // all failed results
```

### TauriLink & Interceptors

Build composable middleware chains:

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
    // Built-in helpers
    logging({ prefix: "[RPC]" }),
    retry({ maxRetries: 3, delay: 1000 }),
    onError((error, ctx) => analytics.track("rpc_error", { path: ctx.path })),

    // Custom interceptor
    async (ctx, next) => {
      ctx.meta.startTime = Date.now();
      const result = await next();
      console.log(`${ctx.path} took ${Date.now() - ctx.meta.startTime}ms`);
      return result;
    },
  ],
});

const rpc = createClientFromLink<AppContract>(link);
```

#### Client Context

Pass request-scoped data through the interceptor chain:

```typescript
interface ClientContext {
  requestId: string;
  userId?: string;
}

const link = new TauriLink<ClientContext>({
  interceptors: [
    async (ctx, next) => {
      console.log(`Request ${ctx.context.requestId} by ${ctx.context.userId}`);
      return next();
    },
  ],
});

const rpc = createClientFromLink<AppContract, ClientContext>(link);

await rpc.user.get(
  { id: 1 },
  {
    context: { requestId: "req-123", userId: "user-456" },
  },
);
```

### Subscriptions

Real-time streaming with async iterators:

```typescript
import { subscribe, createEventIterator } from "@tauri-nexus/rpc-core";

// Using the subscribe helper
const stream = await subscribe<NotificationEvent>(
  "notifications.subscribe",
  {},
);

for await (const event of stream) {
  console.log("New notification:", event);
}

// Manual cleanup
await stream.return();
```

### Zod Schema Validation

Define contracts with runtime validation:

```typescript
import { z } from "zod";
import {
  procedure,
  router,
  createValidatedClient,
  TauriLink,
} from "@tauri-nexus/rpc-core";

const contract = router({
  user: router({
    get: procedure()
      .input(z.object({ id: z.number().positive() }))
      .output(
        z.object({
          id: z.number(),
          name: z.string(),
          email: z.string().email(),
        }),
      )
      .query(),

    create: procedure()
      .input(
        z.object({
          name: z.string().min(1).max(100),
          email: z.string().email(),
        }),
      )
      .output(z.object({ id: z.number(), name: z.string(), email: z.string() }))
      .mutation(),
  }),
});

const link = new TauriLink();
const rpc = createValidatedClient(contract, link, {
  validateInput: true, // Validate inputs before sending
  validateOutput: true, // Validate responses from backend
  strict: false, // Allow extra keys (strip them)
});

// Invalid input throws VALIDATION_ERROR before the call is made
await rpc.user.create({ name: "", email: "invalid" });
// => RpcError { code: "VALIDATION_ERROR", details: { issues: [...] } }
```

## API Reference

### Client Creation

| Function                                        | Description                               |
| ----------------------------------------------- | ----------------------------------------- |
| `createClient<T>()`                             | Create a basic RPC client                 |
| `createClientWithSubscriptions<T>(config)`      | Create a client with subscription support |
| `createClientFromLink<T>(link)`                 | Create a client from a TauriLink          |
| `createValidatedClient(contract, link, config)` | Create a client with Zod validation       |

### Configuration

```typescript
import { configureRpc, getConfig } from "@tauri-nexus/rpc-core";

configureRpc({
  middleware: [...],           // Global middleware
  subscriptionPaths: [...],    // Paths that are subscriptions
  timeout: 30000,              // Request timeout (ms)
  onRequest: (ctx) => {},      // Called before each request
  onResponse: (ctx) => {},     // Called after successful response
  onError: (ctx, error) => {}, // Called on error
});
```

### Error Handling

```typescript
import { isRpcError, hasErrorCode, createError } from "@tauri-nexus/rpc-core";

try {
  await rpc.user.get({ id: 999 });
} catch (error) {
  if (isRpcError(error)) {
    if (hasErrorCode(error, "NOT_FOUND")) {
      console.log("User not found");
    }
    console.log(error.code, error.message, error.details);
  }
}

// Create custom errors
throw createError("VALIDATION_ERROR", "Invalid input", { field: "email" });
```

### Utilities

```typescript
import {
  getProcedures, // List all registered procedures
  getSubscriptionCount, // Get active subscription count
  sleep, // Promise-based delay
  withRetry, // Retry wrapper with backoff
  withDedup, // Deduplicate concurrent calls
  stableStringify, // Deterministic JSON stringify
} from "@tauri-nexus/rpc-core";
```

## TypeScript Support

This library is written in TypeScript and provides comprehensive type definitions. Key type utilities:

```typescript
import type {
  InferInput, // Extract input type from procedure
  InferOutput, // Extract output type from procedure
  ExtractPaths, // Get all procedure paths as union
  ExtractSubscriptionPaths, // Get subscription paths only
  RpcError, // Error type
  RpcErrorCode, // Error code union
} from "@tauri-nexus/rpc-core";

type UserInput = InferInput<AppContract["user"]["get"]>;
// => { id: number }

type AllPaths = ExtractPaths<AppContract>;
// => "health" | "user.get" | "user.create" | "notifications.subscribe"
```

## Related Packages

- [`@tauri-nexus/rpc-react`](../rpc-react) — React hooks and TanStack Query integration
- [`@tauri-nexus/rpc-docs`](../rpc-docs) — Auto-generated API documentation components

## License

MIT © Tauri Nexus
