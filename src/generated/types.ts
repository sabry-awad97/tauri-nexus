// Auto-generated TypeScript types - DO NOT EDIT
// Re-run `cargo build` in src-tauri to regenerate

export interface User {
  id: number;
  name: string;
  email: string;
  createdAt: string;
}

export interface CreateUserInput {
  name: string;
  email: string;
}

export interface UpdateUserInput {
  id: number;
  name?: string;
  email?: string;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  totalPages: number;
}

export interface SuccessResponse {
  success: boolean;
  message?: string;
}

export interface PaginationInput {
  page?: number;
  limit?: number;
}
