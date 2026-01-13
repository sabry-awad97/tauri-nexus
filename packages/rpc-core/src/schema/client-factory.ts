// =============================================================================
// @tauri-nexus/rpc-core - Schema Client Factory
// =============================================================================
// Factory functions for creating validated clients from schema contracts.

import { TauriLink, createClientFromLink } from "../link";
import type { LinkRouterClient } from "../link/client-factory";
import {
  createClientWithSubscriptions,
  type RpcClient,
} from "../public/factory";
import { type RpcClientConfig } from "../client/config";
import type {
  SchemaContract,
  SchemaContractToContract,
  ValidationConfig,
} from "./types";
import { createValidationInterceptor } from "./validation";
import { extractSubscriptionPaths } from "./path-extraction";

// =============================================================================
// Validated Client Factory
// =============================================================================

/**
 * Create a validated client from a schema contract.
 *
 * @example
 * ```typescript
 * const client = createValidatedClient(contract, link, {
 *   validateInput: true,
 *   validateOutput: true,
 * });
 *
 * const user = await client.user.get({ id: 1 });
 * ```
 */
export function createValidatedClient<
  T extends SchemaContract,
  TContext = unknown,
>(
  contract: T,
  link: TauriLink<TContext>,
  config?: ValidationConfig,
): LinkRouterClient<SchemaContractToContract<T>, TContext> {
  const validationInterceptor = createValidationInterceptor(contract, config);
  const existingInterceptors = link.getConfig().interceptors ?? [];

  const validatedLink = new (link.constructor as typeof TauriLink)<TContext>({
    ...link.getConfig(),
    interceptors: [validationInterceptor, ...existingInterceptors],
  });

  return createClientFromLink<SchemaContractToContract<T>, TContext>(
    validatedLink,
  );
}

// =============================================================================
// Schema-Based Client Factory
// =============================================================================

/** Configuration for createClientFromSchema */
export interface SchemaClientConfig extends Omit<
  RpcClientConfig,
  "subscriptionPaths"
> {
  /** Additional client configuration */
  clientConfig?: Omit<RpcClientConfig, "subscriptionPaths">;
}

/**
 * Create a type-safe RPC client directly from a Zod schema contract.
 * Automatically extracts subscription paths from the schema.
 *
 * @example
 * ```typescript
 * const appContract = router({
 *   health: procedure().output(z.object({ status: z.string() })).query(),
 *   stream: router({
 *     counter: procedure()
 *       .input(z.object({ start: z.number().optional() }))
 *       .output(z.object({ count: z.number() }))
 *       .subscription(),
 *   }),
 * });
 *
 * const rpc = createClientFromSchema(appContract);
 * ```
 */
export function createClientFromSchema<T extends SchemaContract>(
  contract: T,
  config?: SchemaClientConfig,
): RpcClient<SchemaContractToContract<T>> {
  const subscriptionPaths = extractSubscriptionPaths(contract);

  return createClientWithSubscriptions<SchemaContractToContract<T>>({
    subscriptionPaths,
    ...config?.clientConfig,
  });
}
