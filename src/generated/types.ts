// =============================================================================
// RPC Types
// =============================================================================
// Mirror of Rust types in src-tauri/src/rpc/types.rs
// Keep in sync when making changes.

// -----------------------------------------------------------------------------
// Common Types
// -----------------------------------------------------------------------------

/** Paginated response wrapper */
export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  totalPages: number;
}

/** Success response for operations */
export interface SuccessResponse {
  success: boolean;
  message?: string;
}

/** Pagination input */
export interface PaginationInput {
  page?: number;
  limit?: number;
}

/** RPC error from backend */
export interface RpcError {
  code: string;
  message: string;
  details?: unknown;
}

// -----------------------------------------------------------------------------
// User Types
// -----------------------------------------------------------------------------

/** User entity */
export interface User {
  id: number;
  name: string;
  email: string;
  createdAt: string;
}

/** Input for getting a user */
export interface GetUserInput {
  id: number;
}

/** Input for creating a user */
export interface CreateUserInput {
  name: string;
  email: string;
}

/** Input for updating a user */
export interface UpdateUserInput {
  id: number;
  name?: string;
  email?: string;
}

/** Input for deleting a user */
export interface DeleteUserInput {
  id: number;
}

// -----------------------------------------------------------------------------
// General Types
// -----------------------------------------------------------------------------

/** Greet input */
export interface GreetInput {
  name: string;
}

/** Health check response */
export interface HealthResponse {
  status: string;
  version: string;
}
