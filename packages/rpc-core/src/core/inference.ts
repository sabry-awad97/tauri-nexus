// =============================================================================
// @tauri-nexus/rpc-core - Type Inference Utilities
// =============================================================================
// Advanced TypeScript type utilities for contract inference.

import type {
  ProcedureDef,
  ProcedureType,
  QueryDef,
  MutationDef,
  SubscriptionDef,
  RpcError,
  CallOptions,
  SubscriptionOptions,
  EventIterator,
} from "./types";

// =============================================================================
// Type Inference Utilities
// =============================================================================

/** Extract input type from procedure */
export type InferInput<T> =
  T extends ProcedureDef<ProcedureType, infer I, unknown> ? I : never;

/** Extract output type from procedure */
export type InferOutput<T> =
  T extends ProcedureDef<ProcedureType, unknown, infer O> ? O : never;

/** Extract procedure type */
export type InferProcedureType<T> =
  T extends ProcedureDef<infer P, unknown, unknown> ? P : never;

/** Check if procedure is a query */
export type IsQuery<T> = T extends QueryDef<unknown, unknown> ? true : false;

/** Check if procedure is a mutation */
export type IsMutation<T> =
  T extends MutationDef<unknown, unknown> ? true : false;

/** Check if procedure is a subscription */
export type IsSubscription<T> =
  T extends SubscriptionDef<unknown, unknown> ? true : false;

// =============================================================================
// Path Extraction Types
// =============================================================================

/** Check if type is a procedure definition (internal helper) */
type IsProcedureType<T> = T extends {
  type: ProcedureType;
  input: unknown;
  output: unknown;
}
  ? true
  : false;

/** Extract all procedure paths from a router */
export type ExtractPaths<T, Prefix extends string = ""> =
  IsProcedureType<T> extends true
    ? Prefix
    : T extends object
      ? {
          [K in keyof T]: K extends string
            ? ExtractPaths<T[K], Prefix extends "" ? K : `${Prefix}.${K}`>
            : never;
        }[keyof T]
      : never;

/** Extract subscription paths from a router */
export type ExtractSubscriptionPaths<
  T,
  Prefix extends string = "",
> = T extends { type: "subscription"; input: unknown; output: unknown }
  ? Prefix
  : T extends object
    ? {
        [K in keyof T]: K extends string
          ? ExtractSubscriptionPaths<
              T[K],
              Prefix extends "" ? K : `${Prefix}.${K}`
            >
          : never;
      }[keyof T]
    : never;

/** Get procedure at a specific path */
export type GetProcedureAtPath<
  T,
  Path extends string,
> = Path extends `${infer Head}.${infer Tail}`
  ? Head extends keyof T
    ? GetProcedureAtPath<T[Head], Tail>
    : never
  : Path extends keyof T
    ? T[Path]
    : never;

// =============================================================================
// Type-Safe Batch Types
// =============================================================================

/**
 * Helper to check if a type is a callable (non-subscription) procedure
 */
type IsCallableProcedure<T> = T extends {
  type: "query" | "mutation";
  input: unknown;
  output: unknown;
}
  ? true
  : false;

/**
 * Helper to get known keys (excluding index signatures)
 */
type KnownKeys<T> = {
  [K in keyof T]: string extends K ? never : number extends K ? never : K;
}[keyof T];

/**
 * Extract all callable (non-subscription) procedure paths from a contract.
 * Subscriptions cannot be batched.
 */
export type ExtractCallablePaths<T> = T extends object
  ? {
      [K in KnownKeys<T>]: K extends string
        ? IsCallableProcedure<T[K]> extends true
          ? K
          : T[K] extends object
            ? T[K] extends {
                type: ProcedureType;
                input: unknown;
                output: unknown;
              }
              ? never
              : {
                  [K2 in KnownKeys<T[K]>]: K2 extends string
                    ? IsCallableProcedure<T[K][K2]> extends true
                      ? `${K}.${K2}`
                      : T[K][K2] extends object
                        ? T[K][K2] extends {
                            type: ProcedureType;
                            input: unknown;
                            output: unknown;
                          }
                          ? never
                          : {
                              [K3 in KnownKeys<T[K][K2]>]: K3 extends string
                                ? IsCallableProcedure<T[K][K2][K3]> extends true
                                  ? `${K}.${K2}.${K3}`
                                  : never
                                : never;
                            }[KnownKeys<T[K][K2]>]
                        : never
                    : never;
                }[KnownKeys<T[K]>]
            : never
        : never;
    }[KnownKeys<T>]
  : never;

/**
 * Get the input type for a procedure at a given path.
 */
export type GetInputAtPath<
  T,
  Path extends string,
> = Path extends `${infer L1}.${infer L2}.${infer L3}`
  ? L1 extends keyof T
    ? T[L1] extends object
      ? L2 extends keyof T[L1]
        ? T[L1][L2] extends object
          ? L3 extends keyof T[L1][L2]
            ? T[L1][L2][L3] extends {
                type: ProcedureType;
                input: infer I;
                output: unknown;
              }
              ? I
              : never
            : never
          : never
        : never
      : never
    : never
  : Path extends `${infer L1}.${infer L2}`
    ? L1 extends keyof T
      ? T[L1] extends object
        ? L2 extends keyof T[L1]
          ? T[L1][L2] extends {
              type: ProcedureType;
              input: infer I;
              output: unknown;
            }
            ? I
            : never
          : never
        : never
      : never
    : Path extends keyof T
      ? T[Path] extends { type: ProcedureType; input: infer I; output: unknown }
        ? I
        : never
      : never;

/**
 * Get the output type for a procedure at a given path.
 */
export type GetOutputAtPath<
  T,
  Path extends string,
> = Path extends `${infer L1}.${infer L2}.${infer L3}`
  ? L1 extends keyof T
    ? T[L1] extends object
      ? L2 extends keyof T[L1]
        ? T[L1][L2] extends object
          ? L3 extends keyof T[L1][L2]
            ? T[L1][L2][L3] extends {
                type: ProcedureType;
                input: unknown;
                output: infer O;
              }
              ? O
              : never
            : never
          : never
        : never
      : never
    : never
  : Path extends `${infer L1}.${infer L2}`
    ? L1 extends keyof T
      ? T[L1] extends object
        ? L2 extends keyof T[L1]
          ? T[L1][L2] extends {
              type: ProcedureType;
              input: unknown;
              output: infer O;
            }
            ? O
            : never
          : never
        : never
      : never
    : Path extends keyof T
      ? T[Path] extends { type: ProcedureType; input: unknown; output: infer O }
        ? O
        : never
      : never;

/**
 * Type-safe batch result that preserves the output type based on the request.
 */
export interface TypedBatchResult<TOutput = unknown> {
  /** The ID of the request this result corresponds to */
  readonly id: string;
  /** The result data (present on success) */
  readonly data?: TOutput;
  /** The error (present on failure) */
  readonly error?: RpcError;
}

/**
 * Helper type to create a request entry for the batch builder.
 */
export type BatchRequestEntry<TId extends string, TOutput> = {
  id: TId;
  output: TOutput;
};

// =============================================================================
// Client Method Types
// =============================================================================

/** Input type handling - void inputs don't require arguments */
type InputArg<TInput> = TInput extends void | undefined | never
  ? []
  : [input: TInput];

/** Options argument type */
type OptionsArg<TOptions> = [options?: TOptions];

/** Convert procedure def to client method signature */
export type ProcedureClient<T> = T extends {
  type: "subscription";
  input: infer I;
  output: infer O;
}
  ? (
      ...args: [...InputArg<I>, ...OptionsArg<SubscriptionOptions>]
    ) => Promise<EventIterator<O>>
  : T extends { type: "query" | "mutation"; input: infer I; output: infer O }
    ? (...args: [...InputArg<I>, ...OptionsArg<CallOptions>]) => Promise<O>
    : never;

/** Check if type is a procedure definition */
type IsProcedureDefinition<T> = T extends {
  type: ProcedureType;
  input: unknown;
  output: unknown;
}
  ? true
  : false;

/** Convert contract router to client type recursively */
export type RouterClient<T> = {
  [K in keyof T]: IsProcedureDefinition<T[K]> extends true
    ? ProcedureClient<T[K]>
    : T[K] extends object
      ? RouterClient<T[K]>
      : never;
};

// =============================================================================
// Client Type Inference Utilities
// =============================================================================

/**
 * Recursively infers the input types from a contract.
 */
export type InferClientInputs<T> = {
  [K in keyof T]: T[K] extends {
    type: ProcedureType;
    input: infer I;
    output: unknown;
  }
    ? I
    : T[K] extends object
      ? InferClientInputs<T[K]>
      : never;
};

/**
 * Recursively infers the output types from a contract.
 */
export type InferClientOutputs<T> = {
  [K in keyof T]: T[K] extends {
    type: ProcedureType;
    input: unknown;
    output: infer O;
  }
    ? O
    : T[K] extends object
      ? InferClientOutputs<T[K]>
      : never;
};

/**
 * Recursively infers the body input types from a contract.
 */
export type InferClientBodyInputs<T> = {
  [K in keyof T]: T[K] extends {
    type: ProcedureType;
    input: infer I;
    output: unknown;
  }
    ? I extends { body: infer B }
      ? B
      : I
    : T[K] extends object
      ? InferClientBodyInputs<T[K]>
      : never;
};

/**
 * Recursively infers the body output types from a contract.
 */
export type InferClientBodyOutputs<T> = {
  [K in keyof T]: T[K] extends {
    type: ProcedureType;
    input: unknown;
    output: infer O;
  }
    ? O extends { body: infer B }
      ? B
      : O
    : T[K] extends object
      ? InferClientBodyOutputs<T[K]>
      : never;
};

/**
 * Recursively infers the error types from a contract.
 */
export type InferClientErrors<T> = {
  [K in keyof T]: T[K] extends {
    type: ProcedureType;
    input: unknown;
    output: unknown;
  }
    ? RpcError
    : T[K] extends object
      ? InferClientErrors<T[K]>
      : never;
};

/**
 * Recursively infers a union of all error types from a contract.
 */
export type InferClientErrorUnion<T> = T extends {
  type: ProcedureType;
  input: unknown;
  output: unknown;
}
  ? RpcError
  : T extends object
    ? { [K in keyof T]: InferClientErrorUnion<T[K]> }[keyof T]
    : never;

/**
 * Infers the procedure type (query, mutation, subscription) for each endpoint.
 */
export type InferClientProcedureTypes<T> = {
  [K in keyof T]: T[K] extends {
    type: infer P;
    input: unknown;
    output: unknown;
  }
    ? P
    : T[K] extends object
      ? InferClientProcedureTypes<T[K]>
      : never;
};

/**
 * Extract all input types as a union from a contract.
 */
export type InferClientInputUnion<T> = T extends {
  type: ProcedureType;
  input: infer I;
  output: unknown;
}
  ? I
  : T extends object
    ? { [K in keyof T]: InferClientInputUnion<T[K]> }[keyof T]
    : never;

/**
 * Extract all output types as a union from a contract.
 */
export type InferClientOutputUnion<T> = T extends {
  type: ProcedureType;
  input: unknown;
  output: infer O;
}
  ? O
  : T extends object
    ? { [K in keyof T]: InferClientOutputUnion<T[K]> }[keyof T]
    : never;

/**
 * Infer the client context type from a client.
 */
export type InferClientContext<T> = T extends { __context?: infer C }
  ? C
  : unknown;
