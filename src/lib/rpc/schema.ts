// =============================================================================
// Zod Schema Validation for RPC Client
// =============================================================================
// Provides Zod-based schema validation with automatic TypeScript type inference.
// Similar to tRPC/oRPC patterns for defining type-safe RPC contracts.

import { z } from "zod";
import type { ProcedureType, RpcError } from "./types";
import type { LinkInterceptor, LinkRequestContext, TauriLink, LinkRouterClient } from "./link";
import { createClientFromLink } from "./link";

// =============================================================================
// Core Schema Types
// =============================================================================

/**
 * Schema-based procedure definition.
 * Contains Zod schemas for input/output validation with automatic type inference.
 */
export interface SchemaProcedure<
  TType extends ProcedureType = ProcedureType,
  TInputSchema extends z.ZodTypeAny | null = z.ZodTypeAny | null,
  TOutputSchema extends z.ZodTypeAny = z.ZodTypeAny,
> {
  readonly type: TType;
  readonly inputSchema: TInputSchema;
  readonly outputSchema: TOutputSchema;
}

/**
 * Schema-based contract router.
 * Can contain procedures or nested routers.
 */
export type SchemaContract = {
  [key: string]: SchemaProcedure<ProcedureType, z.ZodTypeAny | null, z.ZodTypeAny> | SchemaContract;
};

// =============================================================================
// Type Inference Utilities
// =============================================================================

/** Infer input type from a schema procedure */
export type InferSchemaInput<T> =
  T extends SchemaProcedure<ProcedureType, infer I, z.ZodTypeAny>
    ? I extends z.ZodTypeAny
      ? z.input<I>
      : void
    : never;

/** Infer output type from a schema procedure */
export type InferSchemaOutput<T> =
  T extends SchemaProcedure<ProcedureType, z.ZodTypeAny | null, infer O>
    ? z.output<O>
    : never;

/** Infer procedure type from a schema procedure */
export type InferSchemaProcedureType<T> =
  T extends SchemaProcedure<infer P, z.ZodTypeAny | null, z.ZodTypeAny> ? P : never;

/** Infer all input types from a schema contract */
export type InferContractInputs<T> = {
  [K in keyof T]: T[K] extends SchemaProcedure<ProcedureType, infer I, z.ZodTypeAny>
    ? I extends z.ZodTypeAny
      ? z.input<I>
      : void
    : T[K] extends object
      ? InferContractInputs<T[K]>
      : never;
};

/** Infer all output types from a schema contract */
export type InferContractOutputs<T> = {
  [K in keyof T]: T[K] extends SchemaProcedure<ProcedureType, z.ZodTypeAny | null, infer O>
    ? z.output<O>
    : T[K] extends object
      ? InferContractOutputs<T[K]>
      : never;
};

/** Infer input type from a single procedure */
export type InferProcedureInput<T> =
  T extends SchemaProcedure<ProcedureType, infer I, z.ZodTypeAny>
    ? I extends z.ZodTypeAny
      ? z.input<I>
      : void
    : never;

/** Infer output type from a single procedure */
export type InferProcedureOutput<T> =
  T extends SchemaProcedure<ProcedureType, z.ZodTypeAny | null, infer O>
    ? z.output<O>
    : never;

// =============================================================================
// Contract Conversion Types
// =============================================================================

/** Convert schema contract to standard contract for client typing */
export type SchemaContractToContract<T> = {
  [K in keyof T]: T[K] extends SchemaProcedure<infer Type, infer I, infer O>
    ? {
        type: Type;
        input: I extends z.ZodTypeAny ? z.input<I> : void;
        output: z.output<O>;
      }
    : T[K] extends object
      ? SchemaContractToContract<T[K]>
      : never;
};

// =============================================================================
// Procedure Builder
// =============================================================================

/**
 * Fluent builder for creating schema-validated procedures.
 * 
 * @example
 * ```typescript
 * const getUserProcedure = procedure()
 *   .input(z.object({ id: z.number() }))
 *   .output(z.object({ id: z.number(), name: z.string() }))
 *   .query();
 * ```
 */
export class ProcedureBuilder<
  TInputSchema extends z.ZodTypeAny | null = null,
  TOutputSchema extends z.ZodTypeAny | null = null,
> {
  private _inputSchema: TInputSchema;
  private _outputSchema: TOutputSchema;

  constructor(
    inputSchema: TInputSchema = null as TInputSchema,
    outputSchema: TOutputSchema = null as TOutputSchema
  ) {
    this._inputSchema = inputSchema;
    this._outputSchema = outputSchema;
  }

  /**
   * Define the input schema for this procedure.
   * The TypeScript input type will be inferred from the Zod schema.
   */
  input<T extends z.ZodTypeAny>(schema: T): ProcedureBuilder<T, TOutputSchema> {
    return new ProcedureBuilder(schema, this._outputSchema);
  }

  /**
   * Define the output schema for this procedure.
   * The TypeScript output type will be inferred from the Zod schema.
   */
  output<T extends z.ZodTypeAny>(schema: T): ProcedureBuilder<TInputSchema, T> {
    return new ProcedureBuilder(this._inputSchema, schema);
  }

  /**
   * Create a query procedure (for reading data).
   * Requires output schema to be defined.
   */
  query(): TOutputSchema extends z.ZodTypeAny
    ? SchemaProcedure<"query", TInputSchema, TOutputSchema>
    : never {
    if (!this._outputSchema) {
      throw new Error("Output schema is required before calling query()");
    }
    return {
      type: "query",
      inputSchema: this._inputSchema,
      outputSchema: this._outputSchema,
    } as TOutputSchema extends z.ZodTypeAny
      ? SchemaProcedure<"query", TInputSchema, TOutputSchema>
      : never;
  }

  /**
   * Create a mutation procedure (for writing data).
   * Requires output schema to be defined.
   */
  mutation(): TOutputSchema extends z.ZodTypeAny
    ? SchemaProcedure<"mutation", TInputSchema, TOutputSchema>
    : never {
    if (!this._outputSchema) {
      throw new Error("Output schema is required before calling mutation()");
    }
    return {
      type: "mutation",
      inputSchema: this._inputSchema,
      outputSchema: this._outputSchema,
    } as TOutputSchema extends z.ZodTypeAny
      ? SchemaProcedure<"mutation", TInputSchema, TOutputSchema>
      : never;
  }

  /**
   * Create a subscription procedure (for streaming data).
   * Requires output schema to be defined.
   */
  subscription(): TOutputSchema extends z.ZodTypeAny
    ? SchemaProcedure<"subscription", TInputSchema, TOutputSchema>
    : never {
    if (!this._outputSchema) {
      throw new Error("Output schema is required before calling subscription()");
    }
    return {
      type: "subscription",
      inputSchema: this._inputSchema,
      outputSchema: this._outputSchema,
    } as TOutputSchema extends z.ZodTypeAny
      ? SchemaProcedure<"subscription", TInputSchema, TOutputSchema>
      : never;
  }
}

/**
 * Start building a new procedure with Zod schema validation.
 * 
 * @example
 * ```typescript
 * const health = procedure()
 *   .output(z.object({ status: z.string() }))
 *   .query();
 * 
 * const createUser = procedure()
 *   .input(z.object({ name: z.string(), email: z.string().email() }))
 *   .output(z.object({ id: z.number(), name: z.string(), email: z.string() }))
 *   .mutation();
 * ```
 */
export function procedure(): ProcedureBuilder {
  return new ProcedureBuilder();
}

// =============================================================================
// Router Utilities
// =============================================================================

/**
 * Create a router from procedures and nested routers.
 * 
 * @example
 * ```typescript
 * const contract = router({
 *   health: procedure().output(z.object({ status: z.string() })).query(),
 *   user: router({
 *     get: procedure()
 *       .input(z.object({ id: z.number() }))
 *       .output(UserSchema)
 *       .query(),
 *   }),
 * });
 * ```
 */
export function router<T extends SchemaContract>(routes: T): T {
  return routes;
}

/**
 * Merge multiple routers into one.
 * Later routers override earlier ones on key conflicts.
 * 
 * @example
 * ```typescript
 * const baseRouter = router({ health: ... });
 * const userRouter = router({ user: ... });
 * const merged = mergeRouters(baseRouter, userRouter);
 * ```
 */
export function mergeRouters<T extends SchemaContract[]>(
  ...routers: T
): T[number] {
  return Object.assign({}, ...routers);
}


// =============================================================================
// Schema Map Builder
// =============================================================================

/** Entry in the schema map */
interface SchemaMapEntry {
  inputSchema: z.ZodTypeAny | null;
  outputSchema: z.ZodTypeAny;
  type: ProcedureType;
}

/** Check if value is a SchemaProcedure */
function isSchemaProcedure(value: unknown): value is SchemaProcedure {
  return (
    typeof value === "object" &&
    value !== null &&
    "type" in value &&
    "outputSchema" in value &&
    typeof (value as SchemaProcedure).type === "string" &&
    ["query", "mutation", "subscription"].includes((value as SchemaProcedure).type)
  );
}

/**
 * Build a map of path -> schemas from a SchemaContract.
 * Handles nested routers recursively.
 */
export function buildSchemaMap(
  contract: SchemaContract,
  prefix: string = ""
): Map<string, SchemaMapEntry> {
  const map = new Map<string, SchemaMapEntry>();

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      map.set(path, {
        inputSchema: value.inputSchema,
        outputSchema: value.outputSchema,
        type: value.type,
      });
    } else if (typeof value === "object" && value !== null) {
      // Nested router - recurse
      const nestedMap = buildSchemaMap(value as SchemaContract, path);
      for (const [nestedPath, entry] of nestedMap) {
        map.set(nestedPath, entry);
      }
    }
  }

  return map;
}

// =============================================================================
// Validation Configuration
// =============================================================================

/** Validation error details */
export interface ValidationErrorDetails {
  type: "input" | "output";
  path: string;
  issues: Array<{
    path: string;
    message: string;
    code: string;
  }>;
}

/** Validation interceptor configuration */
export interface ValidationConfig {
  /** Enable input validation (default: true) */
  validateInput?: boolean;
  /** Enable output validation (default: true) */
  validateOutput?: boolean;
  /** Use strict mode - fail on unknown keys (default: false) */
  strict?: boolean;
  /** Custom error handler for validation failures */
  onValidationError?: (
    error: z.ZodError,
    context: { path: string; type: "input" | "output" }
  ) => void;
}

// =============================================================================
// Validation Error Creation
// =============================================================================

/**
 * Create a validation error with Zod details.
 */
function createValidationError(
  type: "input" | "output",
  path: string,
  error: z.ZodError
): RpcError {
  return {
    code: "VALIDATION_ERROR",
    message: `${type === "input" ? "Input" : "Output"} validation failed for ${path}`,
    details: {
      type,
      path,
      issues: error.issues.map((issue) => ({
        path: issue.path.join("."),
        message: issue.message,
        code: issue.code,
      })),
    } satisfies ValidationErrorDetails,
  };
}

// =============================================================================
// Validation Interceptor
// =============================================================================

/**
 * Create a validation interceptor for a schema contract.
 * Validates inputs before sending and outputs after receiving.
 * 
 * @example
 * ```typescript
 * const interceptor = createValidationInterceptor(contract, {
 *   validateInput: true,
 *   validateOutput: true,
 *   strict: false,
 * });
 * ```
 */
export function createValidationInterceptor<T extends SchemaContract>(
  contract: T,
  config: ValidationConfig = {}
): LinkInterceptor {
  const {
    validateInput = true,
    validateOutput = true,
    strict = false,
    onValidationError,
  } = config;

  // Build schema map for fast lookup
  const schemaMap = buildSchemaMap(contract);

  return async <T>(ctx: LinkRequestContext, next: () => Promise<T>): Promise<T> => {
    const schemas = schemaMap.get(ctx.path);

    // Input validation
    if (validateInput && schemas?.inputSchema && ctx.input !== null && ctx.input !== undefined) {
      const schema = strict
        ? (schemas.inputSchema as z.ZodObject<z.ZodRawShape>).strict?.() ?? schemas.inputSchema
        : schemas.inputSchema;
      const result = schema.safeParse(ctx.input);

      if (!result.success) {
        onValidationError?.(result.error, { path: ctx.path, type: "input" });
        throw createValidationError("input", ctx.path, result.error);
      }

      // Use transformed data
      ctx.input = result.data;
    }

    // Execute the call
    const response = await next();

    // Output validation
    if (validateOutput && schemas?.outputSchema) {
      const schema = strict
        ? (schemas.outputSchema as z.ZodObject<z.ZodRawShape>).strict?.() ?? schemas.outputSchema
        : schemas.outputSchema;
      const result = schema.safeParse(response);

      if (!result.success) {
        onValidationError?.(result.error, { path: ctx.path, type: "output" });
        throw createValidationError("output", ctx.path, result.error);
      }

      return result.data as T;
    }

    return response;
  };
}

// =============================================================================
// Validated Client Factory
// =============================================================================

/**
 * Create a validated client from a schema contract.
 * Automatically validates inputs and outputs against Zod schemas.
 * 
 * @example
 * ```typescript
 * const contract = router({
 *   user: router({
 *     get: procedure()
 *       .input(z.object({ id: z.number() }))
 *       .output(z.object({ id: z.number(), name: z.string() }))
 *       .query(),
 *   }),
 * });
 * 
 * const client = createValidatedClient(contract, link, {
 *   validateInput: true,
 *   validateOutput: true,
 * });
 * 
 * // Fully type-safe with runtime validation
 * const user = await client.user.get({ id: 1 });
 * ```
 */
export function createValidatedClient<
  T extends SchemaContract,
  TContext = unknown,
>(
  contract: T,
  link: TauriLink<TContext>,
  config?: ValidationConfig
): LinkRouterClient<SchemaContractToContract<T>, TContext> {
  // Create validation interceptor
  const validationInterceptor = createValidationInterceptor(contract, config);
  
  // Get existing interceptors
  const existingInterceptors = link.getConfig().interceptors ?? [];

  // Create new link with validation interceptor prepended
  const validatedLink = new (link.constructor as typeof TauriLink)<TContext>({
    ...link.getConfig(),
    interceptors: [validationInterceptor, ...existingInterceptors],
  });

  return createClientFromLink<SchemaContractToContract<T>, TContext>(validatedLink);
}
