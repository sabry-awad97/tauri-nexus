// =============================================================================
// RPC Router
// =============================================================================
// Type-safe procedure definitions matching the Rust router.

import { call } from "./client";
import type {
  User,
  GetUserInput,
  CreateUserInput,
  UpdateUserInput,
  DeleteUserInput,
  GreetInput,
  HealthResponse,
  SuccessResponse,
} from "./types";

// Re-export types and client utilities
export * from "./types";
export { configure, getProcedures, isRpcError, hasErrorCode } from "./client";

// -----------------------------------------------------------------------------
// Root Procedures
// -----------------------------------------------------------------------------

/** Health check */
export const health = () => call<HealthResponse>("health", null);

/** Greet a user */
export const greet = (input: GreetInput) => call<string>("greet", input);

// -----------------------------------------------------------------------------
// User Procedures
// -----------------------------------------------------------------------------

export const user = {
  /** Get user by ID */
  get: (input: GetUserInput) => call<User>("user.get", input),

  /** List all users */
  list: () => call<User[]>("user.list", null),

  /** Create a new user */
  create: (input: CreateUserInput) => call<User>("user.create", input),

  /** Update an existing user */
  update: (input: UpdateUserInput) => call<User>("user.update", input),

  /** Delete a user */
  delete: (input: DeleteUserInput) =>
    call<SuccessResponse>("user.delete", input),
} as const;

// -----------------------------------------------------------------------------
// Flat API (Alternative)
// -----------------------------------------------------------------------------

export const rpc = {
  health,
  greet,
  // User
  getUser: user.get,
  listUsers: user.list,
  createUser: user.create,
  updateUser: user.update,
  deleteUser: user.delete,
} as const;

export type RpcCommands = typeof rpc;
export type CommandName = keyof RpcCommands;
