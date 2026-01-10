import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createClient } from '../client';
import { procedure, router } from '../builder';
import { TauriRPCError } from '../types';

// Create test router
const testRouter = router({
  greet: procedure()
    .command('greet')
    .input<{ name: string }>()
    .output<string>()
    .query(),

  users: router({
    list: procedure()
      .command('list_users')
      .output<{ id: number; name: string }[]>()
      .query(),

    create: procedure()
      .command('create_user')
      .input<{ name: string; email: string }>()
      .output<{ id: number; name: string; email: string }>()
      .mutation(),
  }),
});

describe('createClient', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('calls invoke with correct command and args', async () => {
    vi.mocked(invoke).mockResolvedValue('Hello, World!');

    const client = createClient(testRouter);
    const result = await client.greet({ name: 'World' });

    expect(invoke).toHaveBeenCalledWith('greet', { name: 'World' });
    expect(result).toBe('Hello, World!');
  });

  it('handles nested router calls', async () => {
    const mockUsers = [
      { id: 1, name: 'Alice' },
      { id: 2, name: 'Bob' },
    ];
    vi.mocked(invoke).mockResolvedValue(mockUsers);

    const client = createClient(testRouter);
    const result = await client.users.list();

    expect(invoke).toHaveBeenCalledWith('list_users', {});
    expect(result).toEqual(mockUsers);
  });

  it('handles mutations with input', async () => {
    const mockUser = { id: 1, name: 'Alice', email: 'alice@test.com' };
    vi.mocked(invoke).mockResolvedValue(mockUser);

    const client = createClient(testRouter);
    const result = await client.users.create({ name: 'Alice', email: 'alice@test.com' });

    expect(invoke).toHaveBeenCalledWith('create_user', {
      name: 'Alice',
      email: 'alice@test.com',
    });
    expect(result).toEqual(mockUser);
  });

  it('throws TauriRPCError on invoke failure', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Command failed'));

    const client = createClient(testRouter);

    await expect(client.greet({ name: 'Test' })).rejects.toThrow(TauriRPCError);
  });

  it('calls onError callback on failure', async () => {
    const onError = vi.fn();
    vi.mocked(invoke).mockRejectedValue(new Error('Command failed'));

    const client = createClient(testRouter, { onError });

    await expect(client.greet({ name: 'Test' })).rejects.toThrow();
    expect(onError).toHaveBeenCalledWith(expect.any(TauriRPCError));
  });

  it('returns undefined for non-existent procedures', () => {
    const client = createClient(testRouter);
    // @ts-expect-error - Testing runtime behavior
    expect(client.nonExistent).toBeUndefined();
  });
});
