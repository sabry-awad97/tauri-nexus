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
