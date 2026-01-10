import { describe, it, expect } from 'vitest';
import { createMockClient, mockResponse, flushPromises, createDeferred } from '../testing';
import { procedure, router } from '../builder';

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
  }),
});

describe('createMockClient', () => {
  it('creates mocks for all procedures', () => {
    const { mocks } = createMockClient(testRouter);

    expect(mocks.greet).toBeDefined();
    expect(mocks.users.list).toBeDefined();
  });

  it('mockInvoke routes to correct mock', async () => {
    const { mocks, mockInvoke } = createMockClient(testRouter);

    mocks.greet.mockResolvedValue('Hello, Test!');

    const result = await mockInvoke('greet', { name: 'Test' });

    expect(result).toBe('Hello, Test!');
    expect(mocks.greet).toHaveBeenCalledWith({ name: 'Test' });
  });

  it('resetAll clears all mocks', () => {
    const { mocks, resetAll } = createMockClient(testRouter);

    mocks.greet.mockResolvedValue('test');
    mocks.greet({ name: 'test' });

    expect(mocks.greet).toHaveBeenCalled();

    resetAll();

    expect(mocks.greet).not.toHaveBeenCalled();
  });

  it('supports default delay', async () => {
    const { mocks, mockInvoke } = createMockClient(testRouter, { defaultDelay: 10 });

    mocks.greet.mockResolvedValue('Delayed');

    const start = Date.now();
    await mockInvoke('greet', { name: 'Test' });
    const elapsed = Date.now() - start;

    expect(elapsed).toBeGreaterThanOrEqual(10);
  });

  it('throws error for unknown command', async () => {
    const { mockInvoke } = createMockClient(testRouter);

    await expect(mockInvoke('unknown_command', {})).rejects.toThrow(
      'No mock found for command: unknown_command'
    );
  });
});

describe('mockResponse', () => {
  it('success returns resolved promise', async () => {
    const response = mockResponse({ id: 1 });
    const result = await response.success();
    expect(result).toEqual({ id: 1 });
  });

  it('error returns rejected promise', async () => {
    const response = mockResponse({ id: 1 });
    await expect(response.error('Failed')).rejects.toThrow('Failed');
  });

  it('delay waits before resolving', async () => {
    const response = mockResponse('delayed');
    const start = Date.now();
    await response.delay(50);
    expect(Date.now() - start).toBeGreaterThanOrEqual(50);
  });

  it('sequence cycles through responses', async () => {
    const response = mockResponse(0);
    const getNext = response.sequence([1, 2, 3]);

    expect(await getNext()).toBe(1);
    expect(await getNext()).toBe(2);
    expect(await getNext()).toBe(3);
    expect(await getNext()).toBe(1); // Cycles back
  });
});

describe('flushPromises', () => {
  it('waits for pending promises', async () => {
    let resolved = false;
    Promise.resolve().then(() => { resolved = true; });

    expect(resolved).toBe(false);
    await flushPromises();
    expect(resolved).toBe(true);
  });
});

describe('createDeferred', () => {
  it('creates controllable promise', async () => {
    const { promise, resolve } = createDeferred<string>();

    let result: string | undefined;
    promise.then((v) => { result = v; });

    expect(result).toBeUndefined();

    resolve('done');
    await flushPromises();

    expect(result).toBe('done');
  });

  it('can reject the promise', async () => {
    const { promise, reject } = createDeferred<string>();

    reject(new Error('Failed'));

    await expect(promise).rejects.toThrow('Failed');
  });
});
