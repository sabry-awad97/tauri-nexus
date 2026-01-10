import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { invoke } from '@tauri-apps/api/core';
import { createReactClient } from '../react';
import { procedure, router } from '../builder';

// Test router
const testRouter = router({
  greet: procedure()
    .command('greet')
    .input<{ name: string }>()
    .output<string>()
    .query(),

  createItem: procedure()
    .command('create_item')
    .input<{ title: string }>()
    .output<{ id: number; title: string }>()
    .mutation(),
});

const { Provider, useQuery, useMutation } = createReactClient(testRouter);

// Test components
function QueryComponent({ name }: { name: string }) {
  const { data, isLoading, error } = useQuery('greet', { name });

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;
  return <div data-testid="result">{data}</div>;
}

function MutationComponent() {
  const mutation = useMutation('createItem');

  return (
    <div>
      <button
        onClick={() => mutation.mutate({ title: 'Test Item' })}
        disabled={mutation.isLoading}
      >
        {mutation.isLoading ? 'Creating...' : 'Create'}
      </button>
      {mutation.data && <div data-testid="result">{mutation.data.title}</div>}
      {mutation.error && <div data-testid="error">{mutation.error.message}</div>}
    </div>
  );
}

describe('createReactClient', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  describe('useQuery', () => {
    it('fetches data on mount', async () => {
      vi.mocked(invoke).mockResolvedValue('Hello, Test!');

      render(
        <Provider>
          <QueryComponent name="Test" />
        </Provider>
      );

      expect(screen.getByText('Loading...')).toBeInTheDocument();

      await waitFor(() => {
        expect(screen.getByTestId('result')).toHaveTextContent('Hello, Test!');
      });

      expect(invoke).toHaveBeenCalledWith('greet', { name: 'Test' });
    });

    it('refetches when input changes', async () => {
      vi.mocked(invoke)
        .mockResolvedValueOnce('Hello, First!')
        .mockResolvedValueOnce('Hello, Second!');

      const { rerender } = render(
        <Provider>
          <QueryComponent name="First" />
        </Provider>
      );

      await waitFor(() => {
        expect(screen.getByTestId('result')).toHaveTextContent('Hello, First!');
      });

      rerender(
        <Provider>
          <QueryComponent name="Second" />
        </Provider>
      );

      await waitFor(() => {
        expect(screen.getByTestId('result')).toHaveTextContent('Hello, Second!');
      });
    });

    it('handles errors', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Fetch failed'));

      render(
        <Provider>
          <QueryComponent name="Test" />
        </Provider>
      );

      await waitFor(() => {
        expect(screen.getByText('Error: Fetch failed')).toBeInTheDocument();
      });
    });
  });

  describe('useMutation', () => {
    it('executes mutation on trigger', async () => {
      const user = userEvent.setup();
      vi.mocked(invoke).mockResolvedValue({ id: 1, title: 'Test Item' });

      render(
        <Provider>
          <MutationComponent />
        </Provider>
      );

      await user.click(screen.getByRole('button'));

      await waitFor(() => {
        expect(screen.getByTestId('result')).toHaveTextContent('Test Item');
      });

      expect(invoke).toHaveBeenCalledWith('create_item', { title: 'Test Item' });
    });

    it('shows loading state during mutation', async () => {
      const user = userEvent.setup();
      let resolveInvoke: (value: any) => void;
      vi.mocked(invoke).mockImplementation(
        () => new Promise((resolve) => { resolveInvoke = resolve; })
      );

      render(
        <Provider>
          <MutationComponent />
        </Provider>
      );

      await user.click(screen.getByRole('button'));

      expect(screen.getByText('Creating...')).toBeInTheDocument();

      await act(async () => {
        resolveInvoke!({ id: 1, title: 'Test Item' });
      });

      await waitFor(() => {
        expect(screen.getByText('Create')).toBeInTheDocument();
      });
    });

    it('handles mutation errors', async () => {
      const user = userEvent.setup();
      vi.mocked(invoke).mockRejectedValue(new Error('Create failed'));

      render(
        <Provider>
          <MutationComponent />
        </Provider>
      );

      await user.click(screen.getByRole('button'));

      await waitFor(() => {
        expect(screen.getByTestId('error')).toHaveTextContent('Create failed');
      });
    });
  });
});
