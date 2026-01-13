// =============================================================================
// @tauri-nexus/rpc-core - Schema Types
// =============================================================================
// Type definitions for Zod-based schema validation.

import type { z } from "zod";
import type { ProcedureType, Prettify } from "../core/types";

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
  [key: string]:
    | SchemaProcedure<ProcedureType, z.ZodTypeAny | null, z.ZodTypeAny>
    | SchemaContract;
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
  T extends SchemaProcedure<infer P, z.ZodTypeAny | null, z.ZodTypeAny>
    ? P
    : never;

/** Infer all input types from a schema contract */
export type InferContractInputs<T> = {
  [K in keyof T]: T[K] extends SchemaProcedure<
    ProcedureType,
    infer I,
    z.ZodTypeAny
  >
    ? I extends z.ZodTypeAny
      ? z.input<I>
      : void
    : T[K] extends object
      ? InferContractInputs<T[K]>
      : never;
};

/** Infer all output types from a schema contract */
export type InferContractOutputs<T> = {
  [K in keyof T]: T[K] extends SchemaProcedure<
    ProcedureType,
    z.ZodTypeAny | null,
    infer O
  >
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
// Event Extraction Types
// =============================================================================

/**
 * Type-level extraction of subscription keys from a schema contract.
 */
type IsSubscriptionProcedure<T> =
  T extends SchemaProcedure<"subscription", z.ZodTypeAny | null, z.ZodTypeAny>
    ? true
    : false;

type ExtractSubscriptionKeysL2<T> = {
  [K in keyof T]: IsSubscriptionProcedure<T[K]> extends true
    ? K
    : T[K] extends object
      ? {
          [K2 in keyof T[K]]: IsSubscriptionProcedure<T[K][K2]> extends true
            ? K2
            : never;
        }[keyof T[K]]
      : never;
}[keyof T];

/**
 * Generate the Events object type from a schema contract.
 */
export type ExtractEventsType<T extends SchemaContract> = Prettify<{
  readonly [K in ExtractSubscriptionKeysL2<T> as K extends string
    ? Uppercase<K>
    : never]: K extends string ? K : never;
}>;

/**
 * Extract event name union type from an Events object.
 */
export type InferEventName<T extends Record<string, string>> = T[keyof T];

/**
 * Tauri event payload wrapper type.
 */
export type EventPayload<T> = {
  payload: T;
};

/**
 * Options for extracting event names from subscriptions.
 */
export interface ExtractEventsOptions {
  /**
   * Transform function for converting subscription path to event name.
   */
  transformKey?: (path: string) => string;
  /**
   * Transform function for converting subscription path to event channel name.
   */
  transformValue?: (path: string) => string;
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
    context: { path: string; type: "input" | "output" },
  ) => void;
}
