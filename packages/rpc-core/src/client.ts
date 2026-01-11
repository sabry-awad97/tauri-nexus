// =============================================================================
// Tauri RPC Client - Client Implementation
// =============================================================================
// Type-safe RPC client with middleware support and automatic path generation.

import { invoke } from "@tauri-apps/api/core";
import type {
  RouterClient,
  RpcError,
  RpcErrorCode,
  CallOptions,
  SubscriptionOptions,
  Middleware,
  RequestContext,
  BatchRequest,
  BatchResponse,
  SingleRequest,
  BatchCallOptions,
} from "./types";
import { createEventIterator } from "./event-iterator";

// =============================================================================
// Client Configuration
// =============================================================================

export interface RpcClientConfig {
  /** Middleware stack - executed in order */
  middleware?: Middleware[];
  /** Paths that are subscriptions (for runtime detection) */
  subscriptionPaths?: string[];
  /** Global request timeout in milliseconds */
  timeout?: number;
  /** Called before each request */
  onRequest?: (ctx: RequestContext) => void;
  /** Called after successful response */
  onResponse?: <R>(ctx: RequestContext, data: R) => void;
  /** Called on error */
  onError?: (ctx: RequestContext, error: RpcError) => void;
}

/** Global configuration store */
let globalConfig: RpcClientConfig = {};

/** Configure the RPC client globally */
export function configureRpc(config: RpcClientConfig): void {
  globalConfig = { ...globalConfig, ...config };
}

/** Get current configuration */
export function getConfig(): RpcClientConfig {
  return globalConfig;
}

// =============================================================================
// Input Validation (matches Rust validation)
// =============================================================================

/**
 * Validate procedure path format.
 * Matches the Rust `validate_path` function in plugin.rs.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 */
export function validatePath(path: string): void {
  if (!path) {
    throw createError("VALIDATION_ERROR", "Procedure path cannot be empty");
  }
  if (path.startsWith(".") || path.endsWith(".")) {
    throw createError(
      "VALIDATION_ERROR",
      "Procedure path cannot start or end with a dot",
    );
  }
  if (path.includes("..")) {
    throw createError(
      "VALIDATION_ERROR",
      "Procedure path cannot contain consecutive dots",
    );
  }
  for (const ch of path) {
    if (!/[a-zA-Z0-9_.]/.test(ch)) {
      throw createError(
        "VALIDATION_ERROR",
        `Procedure path contains invalid character: '${ch}'`,
      );
    }
  }
}

// =============================================================================
// Error Handling
// =============================================================================

/** Parse RPC error from backend response */
function parseError(error: unknown, timeoutMs?: number): RpcError {
  // Handle AbortError (from timeout or manual cancellation)
  if (error instanceof Error) {
    if (error.name === "AbortError") {
      // If we have a timeout value, this was a timeout
      if (timeoutMs !== undefined) {
        return {
          code: "TIMEOUT",
          message: `Request timed out after ${timeoutMs}ms`,
          details: { timeoutMs },
        };
      }
      return { code: "CANCELLED", message: "Request was cancelled" };
    }
    return { code: "UNKNOWN", message: error.message };
  }

  // Handle JSON string errors from backend
  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcError(parsed)) {
        return parsed;
      }
      return { code: "UNKNOWN", message: error };
    } catch {
      return { code: "UNKNOWN", message: error };
    }
  }

  // Handle RpcError objects directly
  if (isRpcError(error)) {
    return error;
  }

  // Fallback for unknown error types
  return { code: "UNKNOWN", message: String(error) };
}

/** Check if error is an RPC error */
export function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof (error as RpcError).code === "string" &&
    typeof (error as RpcError).message === "string"
  );
}

/** Check if error has a specific code */
export function hasErrorCode(
  error: unknown,
  code: RpcErrorCode | string,
): boolean {
  return isRpcError(error) && error.code === code;
}

/** Create a typed RPC error */
export function createError(
  code: RpcErrorCode | string,
  message: string,
  details?: unknown,
): RpcError {
  return { code, message, details };
}

// =============================================================================
// Middleware Execution
// =============================================================================

/** Execute middleware chain with error wrapping */
async function executeWithMiddleware<T>(
  ctx: RequestContext,
  fn: () => Promise<T>,
): Promise<T> {
  const middleware = globalConfig.middleware ?? [];

  // Build middleware chain from right to left
  let next = fn;
  for (let i = middleware.length - 1; i >= 0; i--) {
    const mw = middleware[i];
    const currentNext = next;
    const middlewareIndex = i;
    next = async () => {
      try {
        return await mw(ctx, currentNext);
      } catch (error) {
        // Wrap non-RpcError middleware errors
        if (!isRpcError(error)) {
          throw createError(
            "MIDDLEWARE_ERROR",
            error instanceof Error ? error.message : String(error),
            {
              middlewareIndex,
              originalError: error instanceof Error ? error.message : error,
            },
          );
        }
        throw error;
      }
    };
  }

  return next();
}

// =============================================================================
// Core Call Functions
// =============================================================================

/** Make an RPC call (query or mutation) */
export async function call<T>(
  path: string,
  input: unknown = null,
  options?: CallOptions,
): Promise<T> {
  // Validate path format (matches Rust validation)
  validatePath(path);

  const ctx: RequestContext = {
    path,
    input,
    type: "query", // Will be determined by path in practice
    meta: options?.meta,
    signal: options?.signal,
  };

  globalConfig.onRequest?.(ctx);

  const timeoutMs = options?.timeout;

  try {
    const result = await executeWithMiddleware(ctx, async () => {
      // Handle timeout
      if (timeoutMs) {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

        try {
          const result = await invoke<T>("plugin:rpc|rpc_call", {
            path,
            input,
          });
          clearTimeout(timeoutId);
          return result;
        } catch (error) {
          clearTimeout(timeoutId);
          throw error;
        }
      }

      return invoke<T>("plugin:rpc|rpc_call", { path, input });
    });

    globalConfig.onResponse?.(ctx, result);
    return result;
  } catch (error) {
    const rpcError = parseError(error, timeoutMs);
    globalConfig.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

/** Subscribe to a streaming procedure */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions,
): Promise<ReturnType<typeof createEventIterator<T>>> {
  // Validate path format (matches Rust validation)
  validatePath(path);

  const ctx: RequestContext = {
    path,
    input,
    type: "subscription",
    meta: options?.meta,
    signal: options?.signal,
  };

  globalConfig.onRequest?.(ctx);

  try {
    return await createEventIterator<T>(path, input, options);
  } catch (error) {
    const rpcError = parseError(error);
    globalConfig.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

// =============================================================================
// Batch Call Functions (Internal)
// =============================================================================

/**
 * Internal function to execute batch requests.
 * Use `rpc.batch()` for type-safe batch operations.
 */
async function executeBatch<T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions,
): Promise<BatchResponse<T>> {
  // Validate all paths
  for (const req of requests) {
    validatePath(req.path);
  }

  // Ensure input is serialized as null instead of undefined
  const normalizedRequests = requests.map((req) => ({
    ...req,
    input: req.input === undefined ? null : req.input,
  }));

  const batchRequest: BatchRequest = { requests: normalizedRequests };
  const timeoutMs = options?.timeout;

  try {
    // Handle timeout
    if (timeoutMs) {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

      try {
        const result = await invoke<BatchResponse<T>>(
          "plugin:rpc|rpc_call_batch",
          {
            batch: batchRequest,
          },
        );
        clearTimeout(timeoutId);
        return result;
      } catch (error) {
        clearTimeout(timeoutId);
        throw error;
      }
    }

    return await invoke<BatchResponse<T>>("plugin:rpc|rpc_call_batch", {
      batch: batchRequest,
    });
  } catch (error) {
    const rpcError = parseError(error, timeoutMs);
    console.warn(
      `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
      rpcError.details,
    );
    throw rpcError;
  }
}

// =============================================================================
// Type-Safe Batch Builder
// =============================================================================

import type {
  ExtractCallablePaths,
  GetInputAtPath,
  GetOutputAtPath,
  TypedBatchResult,
} from "./types";

/**
 * Internal type to track batch entries with their output types.
 */
interface BatchEntry {
  id: string;
  path: string;
  input: unknown;
}

/**
 * Type map for tracking request IDs to their output types.
 */
type OutputTypeMap = Record<string, unknown>;

/**
 * Type-safe batch builder that infers paths and inputs from a contract.
 *
 * This builder provides full type safety:
 * - Paths are constrained to valid callable procedures (queries/mutations)
 * - Input types are inferred from the path
 * - Output types are tracked per request ID
 *
 * @example
 * ```typescript
 * import { createTypedBatch } from '../lib/rpc';
 * import type { AppContract } from '../rpc/contract';
 *
 * const response = await createTypedBatch<AppContract>()
 *   .add('health-check', 'health', undefined)
 *   .add('user-1', 'user.get', { id: 1 })
 *   .add('greeting', 'greet', { name: 'World' })
 *   .execute();
 *
 * // Results are typed!
 * const healthResult = response.getResult('health-check');
 * if (healthResult.data) {
 *   console.log(healthResult.data.status); // HealthResponse
 * }
 * ```
 */
export class TypedBatchBuilder<
  TContract,
  TOutputMap extends OutputTypeMap = Record<string, never>,
> {
  private entries: BatchEntry[] = [];

  /**
   * Add a type-safe request to the batch.
   *
   * @param id - Unique identifier for this request
   * @param path - Procedure path (autocompleted from contract)
   * @param input - Input data (type inferred from path)
   */
  add<TId extends string, TPath extends ExtractCallablePaths<TContract>>(
    id: TId,
    path: TPath,
    input: GetInputAtPath<TContract, TPath>,
  ): TypedBatchBuilder<
    TContract,
    TOutputMap & Record<TId, GetOutputAtPath<TContract, TPath>>
  > {
    this.entries.push({ id, path, input });
    return this as unknown as TypedBatchBuilder<
      TContract,
      TOutputMap & Record<TId, GetOutputAtPath<TContract, TPath>>
    >;
  }

  /**
   * Get the current requests in the batch.
   */
  getRequests(): SingleRequest[] {
    return this.entries.map((e) => ({
      id: e.id,
      path: e.path,
      input: e.input,
    }));
  }

  /**
   * Get the number of requests in the batch.
   */
  size(): number {
    return this.entries.length;
  }

  /**
   * Clear all requests from the batch.
   */
  clear(): TypedBatchBuilder<TContract, Record<string, never>> {
    this.entries = [];
    return this as unknown as TypedBatchBuilder<
      TContract,
      Record<string, never>
    >;
  }

  /**
   * Execute the batch and return a typed response.
   * @param options - Optional batch call options
   */
  async execute(
    options?: BatchCallOptions,
  ): Promise<TypedBatchResponseWrapper<TOutputMap>> {
    const response = await executeBatch(this.getRequests(), options);
    return new TypedBatchResponseWrapper<TOutputMap>(response);
  }
}

/**
 * Type-safe batch response with helper methods to get typed results.
 */
export class TypedBatchResponseWrapper<TOutputMap extends OutputTypeMap> {
  private resultMap: Map<string, TypedBatchResult<unknown>>;
  private orderedResults: TypedBatchResult<unknown>[];

  constructor(response: BatchResponse<unknown>) {
    this.orderedResults = response.results;
    this.resultMap = new Map();
    for (const result of response.results) {
      this.resultMap.set(result.id, result);
      // Log warnings for failed requests
      if (result.error) {
        console.warn(
          `[RPC] Batch request '${result.id}' failed: ${result.error.code} - ${result.error.message}`,
        );
      }
    }
  }

  /**
   * Get all results in order.
   */
  get results(): TypedBatchResult<unknown>[] {
    return this.orderedResults;
  }

  /**
   * Get a typed result by request ID.
   * The return type is inferred from the batch builder.
   */
  getResult<TId extends keyof TOutputMap & string>(
    id: TId,
  ): TypedBatchResult<TOutputMap[TId]> {
    const result = this.resultMap.get(id);
    if (!result) {
      return {
        id,
        error: { code: "NOT_FOUND", message: `No result found for id: ${id}` },
      };
    }
    return result as TypedBatchResult<TOutputMap[TId]>;
  }

  /**
   * Check if a specific request succeeded.
   */
  isSuccess(id: string): boolean {
    const result = this.resultMap.get(id);
    return result ? !result.error : false;
  }

  /**
   * Check if a specific request failed.
   */
  isError(id: string): boolean {
    const result = this.resultMap.get(id);
    return result ? !!result.error : true;
  }

  /**
   * Get all successful results.
   */
  getSuccessful(): TypedBatchResult<unknown>[] {
    return this.orderedResults.filter((r) => !r.error);
  }

  /**
   * Get all failed results.
   */
  getFailed(): TypedBatchResult<unknown>[] {
    return this.orderedResults.filter((r) => r.error);
  }

  /**
   * Get the count of successful requests.
   */
  get successCount(): number {
    return this.orderedResults.filter((r) => !r.error).length;
  }

  /**
   * Get the count of failed requests.
   */
  get errorCount(): number {
    return this.orderedResults.filter((r) => r.error).length;
  }
}

// Keep old class name as alias for backwards compatibility
export { TypedBatchResponseWrapper as TypedBatchResponse };

// =============================================================================
// Client Types with Batch Support
// =============================================================================

/**
 * Extended client type that includes the batch() method.
 */
export type RpcClient<T> = RouterClient<T> & {
  /**
   * Create a type-safe batch builder for executing multiple requests.
   *
   * @example
   * ```typescript
   * const response = await rpc.batch()
   *   .add('h', 'health', undefined)
   *   .add('u1', 'user.get', { id: 1 })
   *   .add('g', 'greet', { name: 'World' })
   *   .execute();
   *
   * // Results are typed!
   * const health = response.getResult('h');    // TypedBatchResult<HealthResponse>
   * const user = response.getResult('u1');     // TypedBatchResult<User>
   * ```
   */
  batch(): TypedBatchBuilder<T, Record<string, never>>;
};

// =============================================================================
// Client Factory
// =============================================================================

/** Symbol to identify the client proxy */
const CLIENT_PROXY = Symbol("rpc-client-proxy");

/** Check if path is a subscription */
function isSubscriptionPath(path: string): boolean {
  const paths = globalConfig.subscriptionPaths ?? [];
  return paths.includes(path);
}

/** Create a proxy that builds paths and calls the appropriate function */
function createClientProxy<T>(pathParts: string[]): RpcClient<T> {
  const handler = function (
    inputOrOptions?: unknown,
    maybeOptions?: CallOptions | SubscriptionOptions,
  ) {
    const fullPath = pathParts.join(".");

    // Determine if this is a subscription
    if (isSubscriptionPath(fullPath)) {
      return subscribe(
        fullPath,
        inputOrOptions,
        maybeOptions as SubscriptionOptions,
      );
    }

    return call(fullPath, inputOrOptions, maybeOptions as CallOptions);
  };

  return new Proxy(handler as unknown as RpcClient<T>, {
    get(_target, prop: string | symbol) {
      if (prop === CLIENT_PROXY) return true;
      if (typeof prop === "symbol") return undefined;

      // Handle batch() method at root level
      if (prop === "batch" && pathParts.length === 0) {
        return () => new TypedBatchBuilder<T, Record<string, never>>();
      }

      return createClientProxy([...pathParts, prop]);
    },
    apply(_, __, args: unknown[]) {
      const fullPath = pathParts.join(".");

      if (isSubscriptionPath(fullPath)) {
        return subscribe(fullPath, args[0], args[1] as SubscriptionOptions);
      }

      return call(fullPath, args[0], args[1] as CallOptions);
    },
  });
}

// =============================================================================
// Public Client Factories
// =============================================================================

/**
 * Create a type-safe RPC client from a contract definition.
 *
 * @example
 * ```typescript
 * interface MyContract {
 *   health: { type: 'query'; input: void; output: { status: string } };
 *   user: {
 *     get: { type: 'query'; input: { id: number }; output: User };
 *     create: { type: 'mutation'; input: CreateUserInput; output: User };
 *   };
 *   stream: {
 *     events: { type: 'subscription'; input: void; output: Event };
 *   };
 * }
 *
 * const rpc = createClient<MyContract>({
 *   subscriptionPaths: ['stream.events'],
 * });
 *
 * // Full type safety!
 * const health = await rpc.health();
 * const user = await rpc.user.get({ id: 1 });
 * const stream = await rpc.stream.events();
 *
 * // Type-safe batch operations
 * const response = await rpc.batch()
 *   .add('h', 'health', undefined)
 *   .add('u', 'user.get', { id: 1 })
 *   .execute();
 * ```
 */
export function createClient<T>(config?: RpcClientConfig): RpcClient<T> {
  if (config) {
    configureRpc(config);
  }
  return createClientProxy<T>([]);
}

/**
 * Create a client with explicit subscription paths.
 * This is the recommended way to create a client when you have subscriptions.
 *
 * @example
 * ```typescript
 * const rpc = createClientWithSubscriptions<MyContract>({
 *   subscriptionPaths: ['stream.counter', 'stream.chat'],
 *   middleware: [loggingMiddleware],
 * });
 *
 * // Type-safe batch operations
 * const response = await rpc.batch()
 *   .add('h', 'health', undefined)
 *   .execute();
 * ```
 */
export function createClientWithSubscriptions<T>(
  config: RpcClientConfig & { subscriptionPaths: string[] },
): RpcClient<T> {
  configureRpc(config);
  return createClientProxy<T>([]);
}

// =============================================================================
// Utility Exports
// =============================================================================

/** Get list of available procedures from backend */
export async function getProcedures(): Promise<string[]> {
  return invoke<string[]>("plugin:rpc|rpc_procedures");
}

/** Get current subscription count from backend */
export async function getSubscriptionCount(): Promise<number> {
  return invoke<number>("plugin:rpc|rpc_subscription_count");
}

// Re-export for convenience
export { createEventIterator } from "./event-iterator";
