# @tauri-nexus/rpc-effect

Effect-based RPC library for type-safe error handling and composition.

## Overview

This package provides a pure Effect-based implementation of the RPC system. It's designed for developers who want to leverage Effect's powerful features for error handling, dependency injection, and composition.

## Installation

```bash
bun add @tauri-nexus/rpc-effect effect
```

## Usage

### Basic Effect Usage

```typescript
import { Effect, pipe } from "effect";
import { call, makeRpcLayer, RpcCallError } from "@tauri-nexus/rpc-effect";

// Create a transport (provided by rpc-core for Tauri)
const transport = {
  call: async <T>(path: string, input: unknown) => {
    // Your transport implementation
  },
  subscribe: async <T>(path: string, input: unknown) => {
    // Your subscription implementation
  },
};

// Create the layer
const layer = makeRpcLayer(transport);

// Make a call
const program = pipe(
  call<User>("user.get", { id: 1 }),
  Effect.catchTag("RpcCallError", (e) => Effect.succeed({ fallback: true })),
  Effect.provide(layer),
);

const result = await Effect.runPromise(program);
```

### Using the EffectLink

```typescript
import { EffectLink, loggingInterceptor } from "@tauri-nexus/rpc-effect";

const link = new EffectLink({
  subscriptionPaths: ["stream.events"],
  timeout: 5000,
  interceptors: [loggingInterceptor()],
  debug: true,
});

// Set transport
link.setTransport(() => ({
  call: async (path, input) => invoke("plugin:rpc|rpc_call", { path, input }),
  subscribe: async (path, input, options) =>
    createEventIterator(path, input, options),
}));

// Use the link
const user = await link.runCall<User>("user.get", { id: 1 });
```

### Error Handling with Pattern Matching

```typescript
import { matchError, RpcEffectError } from "@tauri-nexus/rpc-effect";

const handleError = (error: RpcEffectError) =>
  matchError(error, {
    onCallError: (e) => `Call failed: ${e.code} - ${e.message}`,
    onTimeoutError: (e) => `Timeout after ${e.timeoutMs}ms`,
    onCancelledError: (e) => `Cancelled: ${e.reason}`,
    onValidationError: (e) => `Validation: ${e.issues.length} issues`,
    onNetworkError: (e) => `Network error: ${e.originalError}`,
  });
```

### Retry with Effect

```typescript
import { withRetry, call } from "@tauri-nexus/rpc-effect";

const resilientCall = withRetry(call<Data>("api.getData", { id: 1 }), {
  maxRetries: 3,
  baseDelay: 1000,
  backoff: "exponential",
  retryableCodes: ["TIMEOUT", "INTERNAL_ERROR"],
});
```

## API Reference

### Error Types

- `RpcCallError` - General RPC call errors
- `RpcTimeoutError` - Timeout errors
- `RpcCancelledError` - Cancellation errors
- `RpcValidationError` - Input/output validation errors
- `RpcNetworkError` - Network-level errors

### Services

- `RpcConfigService` - Configuration service
- `RpcTransportService` - Transport abstraction
- `RpcInterceptorService` - Interceptor chain
- `RpcLoggerService` - Logging service

### Interceptors

- `loggingInterceptor` - Log all requests/responses
- `retryInterceptor` - Automatic retry with backoff
- `authInterceptor` - Add authentication headers
- `timingInterceptor` - Track request duration
- `dedupeInterceptor` - Deduplicate concurrent requests

## License

MIT
