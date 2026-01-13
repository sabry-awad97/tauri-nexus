// =============================================================================
// @tauri-nexus/rpc-core - Contract Builder Helpers
// =============================================================================
// Helpers for defining type-safe RPC contracts.

import type { QueryDef, MutationDef, SubscriptionDef } from "./types";

/**
 * Define a query procedure.
 */
export function query<TInput = void, TOutput = void>(): QueryDef<
  TInput,
  TOutput
> {
  return {
    _type: "query",
    _input: undefined as TInput,
    _output: undefined as TOutput,
  };
}

/**
 * Define a mutation procedure.
 */
export function mutation<TInput = void, TOutput = void>(): MutationDef<
  TInput,
  TOutput
> {
  return {
    _type: "mutation",
    _input: undefined as TInput,
    _output: undefined as TOutput,
  };
}

/**
 * Define a subscription procedure.
 */
export function subscription<TInput = void, TOutput = void>(): SubscriptionDef<
  TInput,
  TOutput
> {
  return {
    _type: "subscription",
    _input: undefined as TInput,
    _output: undefined as TOutput,
  };
}
