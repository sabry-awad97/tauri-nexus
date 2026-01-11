# @tauri-nexus/rpc-react

> React hooks and TanStack Query integration for Tauri RPC. Build reactive UIs with real-time subscriptions.

[![npm version](https://img.shields.io/npm/v/@tauri-nexus/rpc-react.svg)](https://www.npmjs.com/package/@tauri-nexus/rpc-react)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Features

- ⚛️ **React hooks** — `useSubscription`, `useBatch`, and more
- 🔄 **TanStack Query integration** — Queries, mutations, and cache management
- 📡 **Real-time subscriptions** — Auto-reconnect, error handling, lifecycle management
- 🎯 **Type-safe** — Full TypeScript inference from your contract
- 🔌 **Re-exports core** — Single import for all RPC functionality

## Installation

```bash
npm install @tauri-nexus/rpc-react @tanstack/react-query
# or
pnpm add @tauri-nexus/rpc-react @tanstack/react-query
# or
bun add @tauri-nexus/rpc-react @tanstack/react-query
```

### Peer Dependencies

- `react` ^18.0.0 || ^19.0.0
- `@tanstack/react-query` ^5.0.0

## Quick Start

### 1. Setup Query Client

```tsx
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const queryClient = new QueryClient();

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <YourApp />
    </QueryClientProvider>
  );
}
```

### 2. Create RPC Client & Utils

```typescript
import {
  createClientWithSubscriptions,
  createTanstackQueryUtils,
} from "@tauri-nexus/rpc-react";

// Define your contract
interface AppContract {
  user: {
    get: { type: "query"; input: { id: number }; output: User };
    list: { type: "query"; input: void; output: User[] };
    create: { type: "mutation"; input: CreateUserInput; output: User };
  };
  notifications: {
    stream: { type: "subscription"; input: void; output: Notification };
  };
}

// Create client
export const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: ["notifications.stream"],
});

// Create TanStack Query utilities
export const api = createTanstackQueryUtils<AppContract>(rpc);
```

### 3. Use in Components

```tsx
import { useQuery, useMutation } from "@tanstack/react-query";
import { api, rpc } from "./rpc";

function UserProfile({ userId }: { userId: number }) {
  // Queries with full type inference
  const { data: user, isLoading } = useQuery(
    api.user.get.queryOptions({ input: { id: userId } }),
  );

  // Mutations
  const createUser = useMutation(api.user.create.mutationOptions());

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      <h1>{user?.name}</h1>
      <button
        onClick={() =>
          createUser.mutate({ name: "New User", email: "new@example.com" })
        }
      >
        Create User
      </button>
    </div>
  );
}
```

## React Hooks

### useSubscription

Manage real-time subscriptions with automatic lifecycle handling:

```tsx
import { useSubscription, subscribe } from "@tauri-nexus/rpc-react";

function NotificationFeed() {
  const { data, isConnected, error } = useSubscription<Notification>(
    () => subscribe("notifications.stream", {}),
    [], // dependency array
    {
      onEvent: (notification) => {
        console.log("New notification:", notification);
      },
      onError: (error) => {
        console.error("Subscription error:", error);
      },
      onComplete: () => {
        console.log("Subscription completed");
      },
      enabled: true, // Enable/disable subscription
      autoReconnect: true, // Auto-reconnect on disconnect
      maxReconnects: 5, // Max reconnection attempts
      reconnectDelay: 1000, // Delay between reconnects (ms)
      maxEvents: undefined, // Limit number of events
    },
  );

  return (
    <div>
      <div className={isConnected ? "connected" : "disconnected"}>
        {isConnected ? "🟢 Connected" : "🔴 Disconnected"}
      </div>
      {error && <div className="error">{error.message}</div>}
      {data && <NotificationCard notification={data} />}
    </div>
  );
}
```

#### Subscription State

```typescript
interface SubscriptionResult<T> {
  data: T | null; // Latest event data
  events: T[]; // All received events (if collecting)
  isConnected: boolean; // Connection status
  error: RpcError | null; // Last error
  reconnectCount: number; // Number of reconnections
}
```

### useBatch

Execute multiple RPC calls in a single request:

```tsx
import { useBatch } from "@tauri-nexus/rpc-react";
import { rpc } from "./rpc";

function Dashboard() {
  const batch = useBatch(
    () =>
      rpc
        .batch()
        .add("health", "health", undefined)
        .add("users", "user.list", undefined)
        .add("user1", "user.get", { id: 1 }),
    {
      executeOnMount: true, // Execute immediately on mount
      onSuccess: (response) => {
        console.log(`${response.successCount} calls succeeded`);
      },
      onError: (error) => {
        console.error("Batch failed:", error);
      },
    },
  );

  if (batch.isLoading) return <div>Loading...</div>;
  if (batch.isError) return <div>Error: {batch.error?.message}</div>;

  // Type-safe result access
  const health = batch.getResult("health");
  const users = batch.getResult("users");
  const user1 = batch.getResult("user1");

  return (
    <div>
      <p>Status: {health?.data?.status}</p>
      <p>Users: {users?.data?.length}</p>
      <p>User 1: {user1?.data?.name}</p>
      <p>Duration: {batch.duration}ms</p>
      <button onClick={() => batch.execute()}>Refresh</button>
    </div>
  );
}
```

#### Batch State

```typescript
interface BatchState {
  isLoading: boolean;
  isSuccess: boolean;
  isError: boolean;
  error: RpcError | null;
  duration: number | null;
  response: TypedBatchResponseWrapper | null;

  execute(): Promise<void>;
  reset(): void;
  getResult(id: string): BatchResult | undefined;
}
```

### useIsMounted

Utility hook for safe async state updates:

```tsx
import { useIsMounted } from "@tauri-nexus/rpc-react";

function AsyncComponent() {
  const isMounted = useIsMounted();
  const [data, setData] = useState(null);

  useEffect(() => {
    fetchData().then((result) => {
      if (isMounted()) {
        setData(result); // Safe - won't update unmounted component
      }
    });
  }, []);

  return <div>{data}</div>;
}
```

## TanStack Query Integration

### Query Options

```tsx
import { useQuery, useQueries } from "@tanstack/react-query";

// Single query
const { data } = useQuery(api.user.get.queryOptions({ input: { id: 1 } }));

// Multiple queries
const results = useQueries({
  queries: [
    api.user.get.queryOptions({ input: { id: 1 } }),
    api.user.get.queryOptions({ input: { id: 2 } }),
    api.user.list.queryOptions(),
  ],
});

// With custom options
const { data } = useQuery({
  ...api.user.get.queryOptions({ input: { id: 1 } }),
  staleTime: 5 * 60 * 1000,
  refetchOnWindowFocus: false,
});
```

### Mutation Options

```tsx
import { useMutation, useQueryClient } from "@tanstack/react-query";

function CreateUserForm() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    ...api.user.create.mutationOptions(),
    onSuccess: () => {
      // Invalidate user list cache
      queryClient.invalidateQueries({ queryKey: api.user.list.key() });
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        mutation.mutate({ name: "Alice", email: "alice@example.com" });
      }}
    >
      {/* form fields */}
      <button disabled={mutation.isPending}>
        {mutation.isPending ? "Creating..." : "Create User"}
      </button>
    </form>
  );
}
```

### Infinite Queries

```tsx
import { useInfiniteQuery } from "@tanstack/react-query";

// Assuming your contract has pagination
const { data, fetchNextPage, hasNextPage } = useInfiniteQuery(
  api.user.list.infiniteOptions({
    input: { limit: 10 },
    getNextPageParam: (lastPage) => lastPage.nextCursor,
  }),
);
```

### Cache Keys

```tsx
// Get query key for cache operations
const userKey = api.user.get.key({ id: 1 });
// => ["user", "get", { id: 1 }]

const userListKey = api.user.list.key();
// => ["user", "list"]

// Invalidate specific query
queryClient.invalidateQueries({ queryKey: api.user.get.key({ id: 1 }) });

// Invalidate all user queries
queryClient.invalidateQueries({ queryKey: api.user.key() });
```

### Direct Calls

```tsx
// Call without hooks (useful in event handlers, effects, etc.)
const user = await api.user.get.call({ id: 1 });
const newUser = await api.user.create.call({
  name: "Bob",
  email: "bob@example.com",
});
```

## Custom Subscription Hooks

Create typed subscription hooks for your specific use cases:

```tsx
import { useSubscription, subscribe } from "@tauri-nexus/rpc-react";
import { rpc } from "./rpc";

// Typed counter subscription
export function useCounter(config: { start?: number; maxCount?: number } = {}) {
  return useSubscription<CounterEvent>(
    () => rpc.stream.counter(config),
    [config.start, config.maxCount],
    { autoReconnect: true },
  );
}

// Typed chat subscription
export function useChat(roomId: string) {
  return useSubscription<ChatMessage>(
    () => rpc.stream.chat({ roomId }),
    [roomId],
    {
      autoReconnect: true,
      maxReconnects: 10,
      onEvent: (message) => {
        // Play notification sound, etc.
      },
    },
  );
}

// Usage
function ChatRoom({ roomId }: { roomId: string }) {
  const { data: message, isConnected } = useChat(roomId);
  // ...
}
```

## Re-exported from @tauri-nexus/rpc-core

This package re-exports everything from `@tauri-nexus/rpc-core` for convenience:

```tsx
import {
  // Client creation
  createClient,
  createClientWithSubscriptions,
  createClientFromLink,

  // TauriLink
  TauriLink,
  logging,
  retry,
  onError,

  // Zod validation
  procedure,
  router,
  createValidatedClient,

  // Utilities
  isRpcError,
  hasErrorCode,
  subscribe,

  // Types
  type RpcError,
  type InferInput,
  type InferOutput,
} from "@tauri-nexus/rpc-react";
```

## TypeScript Support

Full type inference from your contract:

```typescript
import type {
  SubscriptionResult,
  SubscriptionHookOptions,
  BatchState,
  UseBatchOptions,
  TanstackQueryUtils,
} from "@tauri-nexus/rpc-react";

// Infer types from your API utils
type UserQueryOptions = ReturnType<typeof api.user.get.queryOptions>;
type CreateUserMutation = ReturnType<typeof api.user.create.mutationOptions>;
```

## Related Packages

- [`@tauri-nexus/rpc-core`](../rpc-core) — Core RPC client (framework-agnostic)
- [`@tauri-nexus/rpc-docs`](../rpc-docs) — Auto-generated API documentation components

## License

MIT © Tauri Nexus
