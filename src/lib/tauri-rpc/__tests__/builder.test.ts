import { describe, it, expect } from 'vitest';
import { procedure, router } from '../builder';

describe('ProcedureBuilder', () => {
  it('creates a query procedure with command', () => {
    const proc = procedure()
      .command('test_command')
      .input<{ id: number }>()
      .output<string>()
      .query();

    expect(proc._command).toBe('test_command');
    expect(proc._type).toBe('query');
  });

  it('creates a mutation procedure', () => {
    const proc = procedure()
      .command('create_item')
      .input<{ name: string }>()
      .output<{ id: number; name: string }>()
      .mutation();

    expect(proc._command).toBe('create_item');
    expect(proc._type).toBe('mutation');
  });

  it('creates procedure without input', () => {
    const proc = procedure()
      .command('get_all')
      .output<string[]>()
      .query();

    expect(proc._command).toBe('get_all');
  });
});

describe('router', () => {
  it('creates a flat router', () => {
    const appRouter = router({
      greet: procedure()
        .command('greet')
        .input<{ name: string }>()
        .output<string>()
        .query(),
      
      create: procedure()
        .command('create')
        .input<{ title: string }>()
        .output<{ id: number }>()
        .mutation(),
    });

    expect(appRouter.greet._command).toBe('greet');
    expect(appRouter.greet._type).toBe('query');
    expect(appRouter.create._command).toBe('create');
    expect(appRouter.create._type).toBe('mutation');
  });

  it('creates nested routers', () => {
    const appRouter = router({
      users: router({
        list: procedure()
          .command('list_users')
          .output<{ id: number; name: string }[]>()
          .query(),
        
        create: procedure()
          .command('create_user')
          .input<{ name: string; email: string }>()
          .output<{ id: number }>()
          .mutation(),
      }),
      
      posts: router({
        getById: procedure()
          .command('get_post')
          .input<{ id: number }>()
          .output<{ id: number; title: string }>()
          .query(),
      }),
    });

    expect(appRouter.users.list._command).toBe('list_users');
    expect(appRouter.users.create._command).toBe('create_user');
    expect(appRouter.posts.getById._command).toBe('get_post');
  });
});
