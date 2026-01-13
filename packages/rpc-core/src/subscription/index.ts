// =============================================================================
// @tauri-nexus/rpc-core - Subscription Module (Internal)
// =============================================================================
// Exports Effect-based APIs only. Promise wrappers are in public/.

export {
  createEventIteratorEffect,
  consumeEventIterator,
  type ConsumeOptions,
} from "./effect-iterator";
