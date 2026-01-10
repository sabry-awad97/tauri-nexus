# Tauri RPC

Type-safe RPC for Tauri v2 with React hooks support.

## Usage

### Define Router

```typescript
// src/rpc/router.ts
import { router, procedure } from '../lib/tauri-rpc';

export const appRouter = router({
  greet: procedure()
    .command('greet')
    .input<{ name: string }>()
    .output<string>()
    .query(),

  users: router({
    list: procedure()
      .command('list_users')
      .output<User[]>()
      .query(),
  }),
});
```

### Vanilla TypeScript

```typescript
import { createClient } from '../lib/tauri-rpc';
import { appRouter } from './router';

const rpc = createClient(appRouter);

// Type-safe calls
const greeting = await rpc.greet({ name: 'World' });
const users = await rpc.users.list();
```

### React Hooks

```tsx
import { createReactClient } from '../lib/tauri-rpc';
import { appRouter } from './router';

const { Provider, useQuery, useMutation } = createReactClient(appRouter);

function App() {
  const { data, isLoading } = useQuery('greet', { name: 'World' });
  const mutation = useMutation('greet');

  return (
    <Provider>
      <div>{isLoading ? 'Loading...' : data}</div>
      <button onClick={() => mutation.mutate({ name: 'Test' })}>
        Greet
      </button>
    </Provider>
  );
}
```

## Testing

### Setup

```typescript
// src/test/setup.ts
import { vi } from 'vitest';
import '@testing-library/jest-dom/vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
```

### Unit Tests

```typescript
import { vi, describe, it, expect } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { render, screen, waitFor } from '@testing-library/react';

describe('MyComponent', () => {
  it('fetches data', async () => {
    vi.mocked(invoke).mockResolvedValue('Hello!');

    render(<MyComponent />);

    await waitFor(() => {
      expect(screen.getByText('Hello!')).toBeInTheDocument();
    });

    expect(invoke).toHaveBeenCalledWith('greet', { name: 'World' });
  });
});
```

### Mock Utilities

```typescript
import { createMockClient, mockResponse, createDeferred } from '../lib/tauri-rpc/testing';

// Create typed mocks
const { mocks, mockInvoke, resetAll } = createMockClient(appRouter);

// Setup responses
mocks.greet.mockResolvedValue('Hello!');
mocks.users.list.mockResolvedValue([{ id: 1, name: 'Alice' }]);

// Response helpers
const response = mockResponse({ id: 1 });
await response.success();        // Resolves immediately
await response.delay(100);       // Resolves after 100ms
await response.error('Failed');  // Rejects with error

// Deferred for async control
const { promise, resolve, reject } = createDeferred<string>();
// Later: resolve('done') or reject(new Error('failed'))
```

## Scripts

```bash
bun run test           # Run tests once
bun run test:watch     # Watch mode
bun run test:coverage  # With coverage report
```
