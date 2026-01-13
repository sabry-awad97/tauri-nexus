// =============================================================================
// @tauri-nexus/rpc-core - Vanilla TypeScript RPC Client
// =============================================================================
// Core RPC client library for Tauri v2 applications.
// No React dependencies - works with any framework.

// Core types, errors, validation, and contract builders
export * from "./core";

// Client factories, configuration, and batch builder
export * from "./client";

// Event iterator for subscriptions
export * from "./subscription";

// TauriLink (oRPC-style Link Abstraction)
export * from "./link";

// Utility functions (retry, dedup, serialization)
export * from "./utils";

// Zod schema validation and procedure builder
export * from "./schema";

// =============================================================================
// Effect-Based API (Advanced)
// =============================================================================
// For users who want Effect's benefits (type-safe errors, composition).
// Most users should use the standard Promise-based API above.
//
// Import from '@tauri-nexus/rpc-core/effect' for the Effect API:
//
// ```typescript
// import { createEffectClient, loggingInterceptor } from '@tauri-nexus/rpc-core/effect';
// ```
