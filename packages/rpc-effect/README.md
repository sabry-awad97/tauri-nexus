# @tauri-nexus/rpc-effect

A pure [Effect](https://effect.website)-based RPC library providing type-safe error handling, functional composition, and robust service architecture for building reliable communication layers.

## Overview

`@tauri-nexus/rpc-effect` is the foundational layer of the Tauri Nexus RPC system. It provides a transport-agnostic, Effect-native API that enables developers to build type-safe, composable, and resilient RPC clients. The library leverages Effect's powerful abstractions for dependency injection, error handling, and resource management.

This package is designed for developers who want full control over their RPC layer using functional programming patterns, or as the underlying engine for higher-level abstractions like `@tauri-nexus/rpc-core`.

## Key Features

### Type-Safe Error Handling

Every error in the system is represented as a discriminated union type, enabling exhaustive pattern matching and compile-time safety. No more `catch (e: any)` — every failure mode is explicitly typed and handled.

### Functional Composition

Built entirely on Effect, the library embraces functional composition. Complex workflows are constructed from simple, reusable building blocks that can be combined, transformed, and extended without mutation or side effects.

### Service-Based Architecture

The library uses Effect's service pattern for dependency injection. Services like `RpcConfigService`, `RpcTransportService`, and `RpcInterceptorService` are provided through layers, making the system highly testable and configurable.

### Interceptor Pipeline

A middleware-style interceptor system allows you to inject cross-cutting concerns like logging, authentication, retry logic, and request deduplication into the request/response lifecycle.

### Transport Agnostic

The library doesn't dictate how you communicate with your backend. Bring your own transport — whether it's Tauri's IPC, HTTP fetch, WebSockets, or any custom protocol.

## Installation

```bash
bun add @tauri-nexus/rpc-effect effect
```

## Core Concepts

### Effects and Layers

All RPC operations return `Effect` values that describe computations without executing them. These effects declare their dependencies (services they require) and their possible failure modes (error types they may produce).

```typescript
import { Effect, pipe } from "effect";
import { call, createRpcLayer } from "@tauri-nexus/rpc-effect";

// Define a transport implementation
const transport = {
  call: async <T>(path: string, input: unknown): Promise<T> => {
    const response = await fetch(`/api/rpc/${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  },
};

// Create a layer that provides all required services
const RpcLayer = createRpcLayer(transport);

// Build a program that makes an RPC call
const fetchUser = pipe(
  call<User>("user.getById", { id: 123 }),
  Effect.tap((user) => Effect.log(`Retrieved user: ${user.email}`)),
  Effect.provide(RpcLayer),
);

// Execute the program
const user = await Effect.runPromise(fetchUser);
```

### Error Types

The library defines five distinct error types, each representing a specific failure mode:

| Error Type           | Description                                                                                                  |
| -------------------- | ------------------------------------------------------------------------------------------------------------ |
| `RpcCallError`       | The RPC call completed but returned an application-level error (e.g., "User not found", "Permission denied") |
| `RpcTimeoutError`    | The request exceeded the configured timeout duration                                                         |
| `RpcCancelledError`  | The request was explicitly cancelled via an AbortSignal                                                      |
| `RpcValidationError` | Input validation failed before the request was sent                                                          |
| `RpcNetworkError`    | A transport-level failure occurred (connection refused, DNS failure, etc.)                                   |

Each error type carries contextual information specific to its failure mode, enabling precise error handling:

```typescript
import { matchError, type RpcEffectError } from "@tauri-nexus/rpc-effect";

function handleError(error: RpcEffectError): string {
  return matchError(error, {
    onCallError: (e) => {
      // Access: e.path, e.code, e.message, e.details
      return `Operation failed: [${e.code}] ${e.message}`;
    },
    onTimeoutError: (e) => {
      // Access: e.path, e.timeoutMs
      return `Request to ${e.path} timed out after ${e.timeoutMs}ms`;
    },
    onCancelledError: (e) => {
      // Access: e.path, e.reason
      return `Request cancelled: ${e.reason}`;
    },
    onValidationError: (e) => {
      // Access: e.path, e.issues (array of validation issues)
      return `Invalid input: ${e.issues.map((i) => i.message).join(", ")}`;
    },
    onNetworkError: (e) => {
      // Access: e.path, e.originalError
      return `Network failure: ${e.originalError}`;
    },
  });
}
```

### Services

The library is built around four core services that can be configured and composed:

**RpcConfigService** — Manages configuration options like default timeout, subscription paths, and other settings.

```typescript
import { RpcConfigService } from "@tauri-nexus/rpc-effect";

const ConfigLayer = RpcConfigService.layer({
  defaultTimeout: 30000,
  subscriptionPaths: new Set(["events.stream", "notifications.subscribe"]),
});
```

**RpcTransportService** — Abstracts the underlying communication mechanism. You provide an implementation that knows how to send requests and receive responses.

```typescript
import { RpcTransportService } from "@tauri-nexus/rpc-effect";

const TransportLayer = RpcTransportService.layer({
  call: async (path, input) => invoke("plugin:rpc|call", { path, input }),
  subscribe: async (path, input, options) =>
    createEventIterator(path, input, options),
});
```

**RpcInterceptorService** — Manages the interceptor chain that processes requests and responses.

```typescript
import {
  RpcInterceptorService,
  loggingInterceptor,
  retryInterceptor,
} from "@tauri-nexus/rpc-effect";

const InterceptorLayer = RpcInterceptorService.withInterceptors([
  loggingInterceptor(),
  retryInterceptor({ maxRetries: 3 }),
]);
```

**RpcLoggerService** — Provides logging capabilities throughout the RPC lifecycle.

```typescript
import { RpcLoggerService, consoleLogger } from "@tauri-nexus/rpc-effect";

const LoggerLayer = RpcLoggerService.layer(consoleLogger);
```

### Interceptors

Interceptors are functions that wrap the request/response cycle, allowing you to inspect, modify, or short-circuit requests. They execute in order, with each interceptor calling `next()` to continue the chain.

**Built-in Interceptors:**

| Interceptor               | Purpose                                                                |
| ------------------------- | ---------------------------------------------------------------------- |
| `loggingInterceptor`      | Logs request start, completion, and errors with configurable verbosity |
| `retryInterceptor`        | Automatically retries failed requests with exponential backoff         |
| `authInterceptor`         | Injects authentication tokens into request metadata                    |
| `timingInterceptor`       | Measures and reports request duration                                  |
| `dedupeInterceptor`       | Prevents duplicate concurrent requests to the same endpoint            |
| `errorHandlerInterceptor` | Transforms or handles specific error types                             |

**Creating Custom Interceptors:**

```typescript
import { createSimpleInterceptor } from "@tauri-nexus/rpc-effect";

const metricsInterceptor = createSimpleInterceptor({
  name: "metrics",
  intercept: async (ctx, next) => {
    const start = performance.now();
    try {
      const result = await next();
      metrics.recordSuccess(ctx.path, performance.now() - start);
      return result;
    } catch (error) {
      metrics.recordFailure(ctx.path, performance.now() - start);
      throw error;
    }
  },
});
```

## High-Level API: EffectLink

For applications that prefer a simpler async/await interface, `EffectLink` provides a high-level wrapper that manages layers internally:

```typescript
import { EffectLink, loggingInterceptor } from "@tauri-nexus/rpc-effect";

const link = new EffectLink({
  timeout: 10000,
  debug: process.env.NODE_ENV === "development",
  interceptors: [loggingInterceptor()],
});

link.setTransport(() => ({
  call: async (path, input) =>
    fetch(`/api/${path}`, {
      method: "POST",
      body: JSON.stringify(input),
    }).then((r) => r.json()),
}));

// Simple async/await usage
const user = await link.call<User>("user.get", { id: 1 });
const users = await link.call<User[]>("user.list", { limit: 10 });
```

## API Reference

### RPC Operations

```typescript
// Execute a single RPC call
call<T>(path: string, input?: unknown, options?: CallOptions): Effect<T, RpcEffectError, RpcServices>

// Subscribe to a streaming endpoint
subscribe<T>(path: string, input?: unknown, options?: SubscribeOptions): Effect<AsyncIterable<T>, RpcEffectError, RpcServices>

// Execute multiple calls in a single batch
batchCall(requests: BatchRequest[], options?: BatchOptions): Effect<BatchResponse, RpcEffectError, RpcServices>
```

### Error Utilities

```typescript
// Constructors
createCallError(path, code, message, details?)
createTimeoutError(path, timeoutMs)
createCancelledError(path, reason)
createValidationError(path, issues)
createNetworkError(path, originalError)

// Type Guards
isRpcCallError(error): error is RpcCallError
isRpcTimeoutError(error): error is RpcTimeoutError
isRpcCancelledError(error): error is RpcCancelledError
isRpcValidationError(error): error is RpcValidationError
isRpcNetworkError(error): error is RpcNetworkError
isEffectRpcError(error): error is RpcEffectError

// Pattern Matching
matchError(error, handlers): T

// Effect Combinators
failWithCallError(path, code, message): Effect<never, RpcCallError>
failWithTimeout(path, timeoutMs): Effect<never, RpcTimeoutError>
failWithValidation(path, issues): Effect<never, RpcValidationError>
failWithNetwork(path, originalError): Effect<never, RpcNetworkError>
failWithCancelled(path, reason): Effect<never, RpcCancelledError>
```

### Resilience Utilities

```typescript
// Retry with configurable backoff
withRetry(effect, {
  maxRetries: 3,
  baseDelay: 1000,
  backoff: "exponential" | "linear" | "constant",
  retryableCodes: ["TIMEOUT", "INTERNAL_ERROR"],
})

// Deduplicate concurrent identical requests
withDedup(key, effect)

// Path validation
validatePath(path): Effect<string, RpcValidationError>
isValidPath(path): boolean
```

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                        Application                             │
├────────────────────────────────────────────────────────────────┤
│                    EffectLink (Optional)                       │
│              High-level async/await interface                  │
├────────────────────────────────────────────────────────────────┤
│                     Effect Operations                          │
│            call() • subscribe() • batchCall()                  │
├────────────────────────────────────────────────────────────────┤
│                    Interceptor Pipeline                        │
│     logging → auth → retry → timing → dedupe → ...             │
├────────────────────────────────────────────────────────────────┤
│                        Services                                │
│   RpcConfigService │ RpcTransportService │ RpcLoggerService    │
├────────────────────────────────────────────────────────────────┤
│                    Transport Layer                             │
│           (Tauri IPC, HTTP, WebSocket, Custom)                 │
└────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── core/           # Error types, type definitions, error utilities
├── services/       # Effect services (Config, Transport, Interceptor, Logger)
├── interceptors/   # Built-in interceptors and composition utilities
├── operations/     # RPC operations (call, subscribe, batch)
├── validation/     # Path and input validation
├── serializable/   # RpcError serialization for cross-boundary transport
├── subscription/   # Subscription state management and reconnection
├── utils/          # Retry, deduplication, timing utilities
├── client/         # EffectLink high-level client
└── index.ts        # Public API exports
```

## License

MIT
