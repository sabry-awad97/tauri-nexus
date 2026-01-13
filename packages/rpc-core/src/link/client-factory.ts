// =============================================================================
// @tauri-nexus/rpc-core - Link Client Factory
// =============================================================================
// Factory for creating type-safe clients from TauriLink.

import type { TauriLink } from "./tauri-link";
import type { LinkCallOptions, LinkSubscribeOptions } from "./types";

// =============================================================================
// Client Types
// =============================================================================

/** Symbol to identify the client proxy */
const CLIENT_PROXY = Symbol("rpc-link-client-proxy");

/** Client with context support */
export type LinkRouterClient<T, TClientContext = unknown> = {
  [K in keyof T]: T[K] extends {
    type: "subscription";
    input: infer I;
    output: infer O;
  }
    ? I extends void
      ? (
          options?: LinkSubscribeOptions<TClientContext>,
        ) => Promise<AsyncIterable<O>>
      : (
          input: I,
          options?: LinkSubscribeOptions<TClientContext>,
        ) => Promise<AsyncIterable<O>>
    : T[K] extends {
          type: "query" | "mutation";
          input: infer I;
          output: infer O;
        }
      ? I extends void
        ? (options?: LinkCallOptions<TClientContext>) => Promise<O>
        : (input: I, options?: LinkCallOptions<TClientContext>) => Promise<O>
      : T[K] extends object
        ? LinkRouterClient<T[K], TClientContext>
        : never;
};

// =============================================================================
// Client Factory
// =============================================================================

/**
 * Create a type-safe RPC client from a TauriLink.
 *
 * @example
 * ```typescript
 * const link = new TauriLink<{ token?: string }>({
 *   interceptors: [authInterceptor],
 * });
 *
 * const client = createClientFromLink<AppContract, { token?: string }>(link);
 *
 * const user = await client.user.get({ id: 1 }, { context: { token: 'abc' } });
 * ```
 */
export function createClientFromLink<T, TClientContext = unknown>(
  link: TauriLink<TClientContext>,
): LinkRouterClient<T, TClientContext> {
  function createProxy(pathParts: string[]): unknown {
    const handler = function (
      inputOrOptions?: unknown,
      maybeOptions?:
        | LinkCallOptions<TClientContext>
        | LinkSubscribeOptions<TClientContext>,
    ) {
      const fullPath = pathParts.join(".");

      const isOptionsObject = (
        obj: unknown,
      ): obj is LinkCallOptions<TClientContext> => {
        if (typeof obj !== "object" || obj === null) return false;
        const keys = Object.keys(obj);
        const optionKeys = ["context", "signal", "timeout", "meta"];
        return keys.length > 0 && keys.every((k) => optionKeys.includes(k));
      };

      let input: unknown = null;
      let options:
        | LinkCallOptions<TClientContext>
        | LinkSubscribeOptions<TClientContext>
        | undefined;

      if (maybeOptions !== undefined) {
        input = inputOrOptions;
        options = maybeOptions;
      } else if (isOptionsObject(inputOrOptions)) {
        input = null;
        options = inputOrOptions;
      } else {
        input = inputOrOptions ?? null;
        options = undefined;
      }

      if (link.isSubscription(fullPath)) {
        return link.subscribe(
          fullPath,
          input,
          options as LinkSubscribeOptions<TClientContext>,
        );
      }

      return link.call(
        fullPath,
        input,
        options as LinkCallOptions<TClientContext>,
      );
    };

    return new Proxy(handler, {
      get(_target, prop: string | symbol) {
        if (prop === CLIENT_PROXY) return true;
        if (typeof prop === "symbol") return undefined;
        return createProxy([...pathParts, prop]);
      },
      apply(_, __, args: unknown[]) {
        const fullPath = pathParts.join(".");

        const isOptionsObject = (
          obj: unknown,
        ): obj is LinkCallOptions<TClientContext> => {
          if (typeof obj !== "object" || obj === null) return false;
          const keys = Object.keys(obj);
          const optionKeys = ["context", "signal", "timeout", "meta"];
          return keys.length > 0 && keys.every((k) => optionKeys.includes(k));
        };

        let input: unknown = null;
        let options:
          | LinkCallOptions<TClientContext>
          | LinkSubscribeOptions<TClientContext>
          | undefined;

        if (args.length >= 2 && args[1] !== undefined) {
          input = args[0];
          options = args[1] as LinkCallOptions<TClientContext>;
        } else if (isOptionsObject(args[0])) {
          input = null;
          options = args[0];
        } else {
          input = args[0] ?? null;
          options = undefined;
        }

        if (link.isSubscription(fullPath)) {
          return link.subscribe(
            fullPath,
            input,
            options as LinkSubscribeOptions<TClientContext>,
          );
        }

        return link.call(
          fullPath,
          input,
          options as LinkCallOptions<TClientContext>,
        );
      },
    });
  }

  return createProxy([]) as LinkRouterClient<T, TClientContext>;
}
