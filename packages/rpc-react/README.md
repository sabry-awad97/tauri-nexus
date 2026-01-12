# @tauri-nexus/rpc-react

> React hooks and TanStack Query integration for Tauri RPC. Build reactive UIs with real-time subscriptions.

[![npm version](https://img.shields.io/npm/v/@tauri-nexus/rpc-react.svg)](https://www.npmjs.com/package/@tauri-nexus/rpc-react)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Features

- ⚛️ **React hooks** — `useSubscription`, `useBatch`, and more
- 🔄 **TanStack Query integration** — Queries, mutations, infinite queries
- 📡 **Real-time subscriptions** — Auto-reconnect, error handling, lifecycle management
- 🎯 **Type-safe** — Full TypeScript inference from your contract
- 🔌 **Re-exports core** — Single import for all RPC functionality

## Installation

```bash
npm install @tauri-nexus/rpc-react @tanstack/react-query
```

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

interface AppContract {
  user: {
    get: { type: "query"; input: { id: number }; output: User };
    create: { type: "mutation"; input: CreateUserInput; output: User };
  };
  notifications: {
    stream: { type: "subscription"; input: void; output: Notification };
  };
}

export const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: ["notifications.stream"],
});

export const api = createTanstackQueryUtils<AppContract>(rpc);
```

### 3. Use in Components

```tsx
import { useQuery, useMutation } from "@tanstack/react-query";
import { useSubscription } from "@tauri-nexus/rpc-react";

function UserProfile({ userId }: { userId: number }) {
  const { data: user } = useQuery(
    api.user.get.queryOptions({ input: { id: userId } })
  );
  const createUser = useMutation(api.user.create.mutationOptions());

  const { data: notification, isConnected } = useSubscription(
    () => rpc.notifications.stream(),
    []
  );

  return <div>{user?.name}</div>;
}
```

---

## React Hooks

### useSubscription

Manage real-time subscriptions with automatic lifecycle handling:

```tsx
import { useSubscription } from "@tauri-nexus/rpc-react";

function NotificationFeed() {
  const {
    data, // Latest event data
    events, // All received events (if collecting)
    isConnected, // Connection status
    error, // Last error
    reconnectCount, // Number of reconnections
  } = useSubscription(
    () => rpc.notifications.stream(),
    [], // dependency array
    {
      enabled: true, // Enable/disable subscription
      autoReconnect: true, // Auto-reconnect on disconnect
      maxReconnects: 5, // Max reconnection attempts
      reconnectDelay: 1000, // Delay between reconnects (ms)
      maxEvents: undefined, // Limit number of events stored
      onEvent: (notification) => {
        console.log("New notification:", notification);
      },
      onError: (error) => {
        console.error("Subscription error:", error);
      },
      onComplete: () => {
        console.log("Subscription completed");
      },
    }
  );

  return (
    <div>
      <span className={isConnected ? "connected" : "disconnected"}>
        {isConnected ? "🟢 Connected" : "🔴 Disconnected"}
      </span>
      {error && <div className="error">{error.message}</div>}
      {data && <NotificationCard notification={data} />}
    </div>
  );
}
```

### useBatch

Execute multiple RPC calls in a single request:

```tsx
import { useBatch } from "@tauri-nexus/rpc-react";

function Dashboard() {
  const batch = useBatch(
    () =>
      rpc
        .batch()
        .add("health", "health", undefined)
        .add("users", "user.list", undefined)
        .add("user1", "user.get", { id: 1 }),
    {
      executeOnMount: true,
      onSuccess: (response) => {
        console.log(`${response.successCount} calls succeeded`);
      },
      onError: (error) => {
        console.error("Batch failed:", error);
      },
    }
  );

  if (batch.isLoading) return <div>Loading...</div>;
  if (batch.isError) return <div>Error: {batch.error?.message}</div>;

  const health = batch.getResult("health");
  const users = batch.getResult("users");

  return (
    <div>
      <p>Status: {health?.data?.status}</p>
      <p>Users: {users?.data?.length}</p>
      <p>Duration: {batch.duration}ms</p>
      <button onClick={() => batch.execute()}>Refresh</button>
    </div>
  );
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
        setData(result);
      }
    });
  }, []);

  return <div>{data}</div>;
}
```

---

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

const { data, fetchNextPage, hasNextPage } = useInfiniteQuery(
  api.user.list.infiniteOptions({
    input: { limit: 10 },
    getNextPageParam: (lastPage) => lastPage.nextCursor,
  })
);
```

### Cache Keys

```tsx
// Get query key for cache operations
const userKey = api.user.get.key({ id: 1 });
// => ["user", "get", { id: 1 }]

// Invalidate specific query
queryClient.invalidateQueries({ queryKey: api.user.get.key({ id: 1 }) });

// Invalidate all user queries
queryClient.invalidateQueries({ queryKey: api.user.key() });
```

### Direct Calls

```tsx
// Call without hooks
const user = await api.user.get.call({ id: 1 });
const newUser = await api.user.create.call({
  name: "Bob",
  email: "bob@example.com",
});
```

---

## Custom Subscription Hooks

Create typed subscription hooks for your specific use cases:

```tsx
import { useSubscription } from "@tauri-nexus/rpc-react";

export function useCounter(config: { start?: number; maxCount?: number } = {}) {
  return useSubscription(
    () => rpc.stream.counter(config),
    [config.start, config.maxCount],
    { autoReconnect: true }
  );
}

export function useChat(roomId: string) {
  return useSubscription(() => rpc.stream.chat({ roomId }), [roomId], {
    autoReconnect: true,
    maxReconnects: 10,
    onEvent: (message) => {
      // Play notification sound
    },
  });
}

// Usage
function ChatRoom({ roomId }: { roomId: string }) {
  const { data: message, isConnected } = useChat(roomId);
  // ...
}
```

---

## Re-exported from @tauri-nexus/rpc-core

This package re-exports everything from `@tauri-nexus/rpc-core`:

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

---

## TypeScript Support

```typescript
import type {
  SubscriptionResult,
  SubscriptionHookOptions,
  BatchState,
  UseBatchOptions,
  TanstackQueryUtils,
} from "@tauri-nexus/rpc-react";

type UserQueryOptions = ReturnType<typeof api.user.get.queryOptions>;
type CreateUserMutation = ReturnType<typeof api.user.create.mutationOptions>;
```

---

## Related Packages

- [`@tauri-nexus/rpc-core`](../rpc-core) — Core RPC client (framework-agnostic)
- [`@tauri-nexus/rpc-docs`](../rpc-docs) — Auto-generated API documentation

## License

MIT © Tauri Nexus
