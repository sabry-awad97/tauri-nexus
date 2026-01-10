// =============================================================================
// Type Definitions
// =============================================================================
// These types mirror the Rust types in src-tauri/src/rpc/types.rs
// Keep them in sync when making changes.

// -----------------------------------------------------------------------------
// Common Types (from tauri-plugin-rpc)
// -----------------------------------------------------------------------------

/** Generic paginated response */
export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  totalPages: number;
}

/** Generic success response */
export interface SuccessResponse {
  success: boolean;
  message?: string;
}

/** Pagination input */
export interface PaginationInput {
  page?: number;
  limit?: number;
}

// -----------------------------------------------------------------------------
// App Types (from src-tauri/src/rpc/types.rs)
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

/** Greet input */
export interface GreetInput {
  name: string;
}
