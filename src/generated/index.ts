// =============================================================================
// RPC Exports
// =============================================================================

// Types
export * from "./types";

// Client utilities
export { configure, getProcedures, isRpcError, hasErrorCode } from "./client";

// Router & procedures
export { rpc, user, health, greet } from "./router";

// React hooks
export {
  RpcProvider,
  useRpc,
  useHealth,
  useGreet,
  useUser,
  useUsers,
  useCreateUser,
  useUpdateUser,
  useDeleteUser,
  type QueryResult,
  type MutationResult,
  type QueryOptions,
  type MutationOptions,
} from "./hooks";
