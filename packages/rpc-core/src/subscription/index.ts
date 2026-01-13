// =============================================================================
// @tauri-nexus/rpc-core - Subscription Module
// =============================================================================

// Promise-based exports (backwards compatible)
export {
  createEventIterator,
  consumeEventIterator,
  type ConsumeOptions,
} from "./effect-iterator";

// Effect-based exports
export { createEventIteratorEffect } from "./effect-iterator";
