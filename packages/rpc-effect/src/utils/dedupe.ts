// =============================================================================
// Deduplication Utilities
// =============================================================================

import { Effect, Ref, HashMap, Option } from "effect";
import type { RpcEffectError } from "../core/errors";
import { createCallError } from "../core/error-utils";
import { stableStringify } from "./serialize";

// =============================================================================
// Cache Factory
// =============================================================================

export const createDedupCache = <A>(): Effect.Effect<{
  withDedup: (
    key: string,
    effect: Effect.Effect<A, RpcEffectError>,
  ) => Effect.Effect<A, RpcEffectError>;
  clear: () => Effect.Effect<void>;
  clearKey: (key: string) => Effect.Effect<void>;
  size: () => Effect.Effect<number>;
}> =>
  Effect.gen(function* () {
    const cacheRef = yield* Ref.make(HashMap.empty<string, Promise<A>>());

    const withDedup = (
      key: string,
      effect: Effect.Effect<A, RpcEffectError>,
    ): Effect.Effect<A, RpcEffectError> =>
      Effect.gen(function* () {
        const cache = yield* Ref.get(cacheRef);
        const existing = HashMap.get(cache, key);

        if (Option.isSome(existing)) {
          return yield* Effect.tryPromise({
            try: () => existing.value,
            catch: (error) =>
              createCallError(
                "DEDUP_ERROR",
                "Deduplicated request failed",
                error,
              ),
          });
        }

        const promise = Effect.runPromise(effect);
        yield* Ref.update(cacheRef, HashMap.set(key, promise));

        try {
          const result = yield* Effect.tryPromise({
            try: () => promise,
            catch: (error) =>
              createCallError("DEDUP_ERROR", "Request failed", error),
          });
          return result;
        } finally {
          yield* Ref.update(cacheRef, HashMap.remove(key));
        }
      });

    const clear = (): Effect.Effect<void> =>
      Ref.set(cacheRef, HashMap.empty<string, Promise<A>>());

    const clearKey = (key: string): Effect.Effect<void> =>
      Ref.update(cacheRef, HashMap.remove(key));

    const size = (): Effect.Effect<number> =>
      Effect.map(Ref.get(cacheRef), HashMap.size);

    return { withDedup, clear, clearKey, size };
  });

// =============================================================================
// Key Generation
// =============================================================================

export const deduplicationKey = (
  path: string,
  input: unknown,
): Effect.Effect<string> =>
  Effect.sync(() => `${path}:${stableStringify(input)}`);

// =============================================================================
// Global Deduplication
// =============================================================================

const globalPendingRequests = new Map<string, Promise<unknown>>();

export const withDedup = <A>(
  key: string,
  effect: Effect.Effect<A, RpcEffectError>,
): Effect.Effect<A, RpcEffectError> =>
  Effect.gen(function* () {
    const existing = globalPendingRequests.get(key);
    if (existing) {
      return yield* Effect.tryPromise({
        try: () => existing as Promise<A>,
        catch: (error) =>
          createCallError("DEDUP_ERROR", "Deduplicated request failed", error),
      });
    }

    const promise = Effect.runPromise(effect);
    globalPendingRequests.set(key, promise);

    try {
      const result = yield* Effect.tryPromise({
        try: () => promise,
        catch: (error) =>
          createCallError("DEDUP_ERROR", "Request failed", error),
      });
      return result;
    } finally {
      globalPendingRequests.delete(key);
    }
  });
