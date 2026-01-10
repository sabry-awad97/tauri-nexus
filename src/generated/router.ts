// =============================================================================
// RPC Router
// =============================================================================
// Type-safe RPC client with path-based routing.
// Calls go through plugin:rpc|rpc_call

import { invoke } from '@tauri-apps/api/core';
import type {
  User,
  GetUserInput,
  CreateUserInput,
  UpdateUserInput,
  DeleteUserInput,
  GreetInput,
  SuccessResponse,
} from './types';

// Re-export all types
export * from './types';

// -----------------------------------------------------------------------------
// RPC Call Helper
// -----------------------------------------------------------------------------

async function call<T>(path: string, input: unknown = {}): Promise<T> {
  return invoke<T>('plugin:rpc|rpc_call', { path, input });
}

/** Get available procedures from the router */
export async function getProcedures(): Promise<string[]> {
  return invoke<string[]>('plugin:rpc|rpc_procedures');
}

// -----------------------------------------------------------------------------
// Procedures
// -----------------------------------------------------------------------------

/** Root-level procedures */
export const greet = (input: GreetInput) => 
  call<string>('greet', input);

/** User procedures (user.*) */
export const user = {
  get: (input: GetUserInput) => call<User>('user.get', input),
  list: () => call<User[]>('user.list', {}),
  create: (input: CreateUserInput) => call<User>('user.create', input),
  update: (input: UpdateUserInput) => call<User>('user.update', input),
  delete: (input: DeleteUserInput) => call<SuccessResponse>('user.delete', input),
} as const;

// -----------------------------------------------------------------------------
// Flat API (for convenience)
// -----------------------------------------------------------------------------

export const rpc = {
  greet,
  getUser: user.get,
  listUsers: user.list,
  createUser: user.create,
  updateUser: user.update,
  deleteUser: user.delete,
} as const;

export type RpcCommands = typeof rpc;
export type CommandName = keyof RpcCommands;
