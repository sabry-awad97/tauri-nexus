// Auto-generated router - Types from ts-rs

import { invoke } from '@tauri-apps/api/core';
import type {
  User,
  CreateUserInput,
  UpdateUserInput,
  PaginatedResponse,
  SuccessResponse,
  PaginationInput,
} from './types';

// Re-export types
export type {
  User,
  CreateUserInput,
  UpdateUserInput,
  PaginatedResponse,
  SuccessResponse,
  PaginationInput,
};

// Command definitions
type Commands = {
  greet: { input: { name: string }; output: string };
  get_user: { input: { id: number }; output: User };
  list_users: { input: { pagination?: PaginationInput }; output: PaginatedResponse };
  create_user: { input: { input: CreateUserInput }; output: User };
  update_user: { input: { input: UpdateUserInput }; output: User };
  delete_user: { input: { id: number }; output: SuccessResponse };
};

/** Type-safe invoke */
async function rpcInvoke<K extends keyof Commands>(
  cmd: K,
  args: Commands[K]['input']
): Promise<Commands[K]['output']> {
  return invoke(cmd, args);
}

// Command functions
export const greet = (input: { name: string }) => rpcInvoke('greet', input);
export const getUser = (input: { id: number }) => rpcInvoke('get_user', input);
export const listUsers = (input: { pagination?: PaginationInput } = {}) => rpcInvoke('list_users', input);
export const createUser = (input: { input: CreateUserInput }) => rpcInvoke('create_user', input);
export const updateUser = (input: { input: UpdateUserInput }) => rpcInvoke('update_user', input);
export const deleteUser = (input: { id: number }) => rpcInvoke('delete_user', input);

/** RPC client object */
export const rpc = {
  greet,
  getUser,
  listUsers,
  createUser,
  updateUser,
  deleteUser,
} as const;

export type CommandName = keyof typeof rpc;
