// =============================================================================
// @tauri-nexus/rpc-react - React + TanStack Query Integration
// =============================================================================
// React hooks and TanStack Query utilities for Tauri RPC.

// Re-export core types and functions for convenience
export * from "@tauri-nexus/rpc-core";

// =============================================================================
// React Hooks
// =============================================================================

export {
  // Subscription hook (TanStack Query doesn't support streaming)
  useSubscription,
  // Batch hook
  useBatch,
  // Utility hooks
  useIsMounted,
  // Types
  type SubscriptionState,
  type SubscriptionResult,
  type SubscriptionHookOptions,
  type BatchState,
  type BatchResult,
  type UseBatchOptions,
} from "./hooks";

// =============================================================================
// TanStack Query Integration
// =============================================================================

export {
  createTanstackQueryUtils,
  type TanstackQueryUtils,
  type CreateTanstackQueryUtilsOptions,
  type QueryOptionsResult,
  type MutationOptionsResult,
  type InfiniteOptionsResult,
  type KeyOptions,
} from "./tanstack";

// Zod Schema Validation is re-exported from rpc-core
