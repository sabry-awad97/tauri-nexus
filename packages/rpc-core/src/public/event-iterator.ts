// =============================================================================
// @tauri-nexus/rpc-core - Event Iterator (Public Promise API)
// =============================================================================
// Promise-based wrappers for subscription event iterators.

import { Effect } from "effect";
import type { EventIterator, SubscriptionOptions } from "../core/types";
import { createEventIteratorEffect } from "../subscription";

/**
 * Create an async event iterator for a subscription.
 */
export async function createEventIterator<T>(
  path: string,
  input: unknown = null,
  options: SubscriptionOptions = {},
): Promise<EventIterator<T>> {
  return Effect.runPromise(createEventIteratorEffect<T>(path, input, options));
}

// Re-export consumeEventIterator and types (already Promise-based)
export { consumeEventIterator, type ConsumeOptions } from "../subscription";
