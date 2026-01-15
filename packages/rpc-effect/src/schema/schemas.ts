// =============================================================================
// Schema Definitions - Effect Schema-based Validation
// =============================================================================

import { Schema } from "effect";
import { Effect, Context, Layer } from "effect";
import type { RpcEffectError } from "../core/errors";
import { createValidationError } from "../core/error-utils";

// =============================================================================
// Core Schemas
// =============================================================================

/**
 * Schema for RPC path validation.
 * Paths must start with a letter and contain only alphanumeric, dots, underscores, or hyphens.
 */
export const RpcPathSchema = Schema.String.pipe(
  Schema.pattern(/^[a-zA-Z][a-zA-Z0-9._-]*$/),
  Schema.annotations({
    identifier: "RpcPath",
    description:
      "RPC procedure path (alphanumeric with dots, underscores, hyphens)",
  }),
);

/**
 * Schema for RPC metadata.
 */
export const RpcMetaSchema = Schema.Record({
  key: Schema.String,
  value: Schema.Unknown,
}).pipe(
  Schema.annotations({
    identifier: "RpcMeta",
    description: "RPC request/response metadata",
  }),
);

/**
 * Schema for RPC request structure.
 */
export const RpcRequestSchema = Schema.Struct({
  path: RpcPathSchema,
  input: Schema.Unknown,
  meta: Schema.optional(RpcMetaSchema),
}).pipe(
  Schema.annotations({
    identifier: "RpcRequest",
    description: "RPC request payload",
  }),
);

/**
 * Schema for RPC response structure.
 */
export const RpcResponseSchema = Schema.Struct({
  data: Schema.Unknown,
  meta: Schema.optional(RpcMetaSchema),
}).pipe(
  Schema.annotations({
    identifier: "RpcResponse",
    description: "RPC response payload",
  }),
);

/**
 * Schema for batch request item.
 */
export const RpcBatchRequestItemSchema = Schema.Struct({
  id: Schema.String,
  path: RpcPathSchema,
  input: Schema.Unknown,
});

/**
 * Schema for batch request.
 */
export const RpcBatchRequestSchema = Schema.Struct({
  requests: Schema.Array(RpcBatchRequestItemSchema),
}).pipe(
  Schema.annotations({
    identifier: "RpcBatchRequest",
    description: "RPC batch request payload",
  }),
);

/**
 * Schema for batch response item.
 */
export const RpcBatchResponseItemSchema = Schema.Struct({
  id: Schema.String,
  data: Schema.optional(Schema.Unknown),
  error: Schema.optional(
    Schema.Struct({
      code: Schema.String,
      message: Schema.String,
      details: Schema.optional(Schema.Unknown),
    }),
  ),
});

/**
 * Schema for batch response.
 */
export const RpcBatchResponseSchema = Schema.Struct({
  results: Schema.Array(RpcBatchResponseItemSchema),
}).pipe(
  Schema.annotations({
    identifier: "RpcBatchResponse",
    description: "RPC batch response payload",
  }),
);

// =============================================================================
// Type Inference
// =============================================================================

export type RpcPath = typeof RpcPathSchema.Type;
export type RpcMeta = typeof RpcMetaSchema.Type;
export type RpcRequest = typeof RpcRequestSchema.Type;
export type RpcResponse = typeof RpcResponseSchema.Type;
export type RpcBatchRequest = typeof RpcBatchRequestSchema.Type;
export type RpcBatchResponse = typeof RpcBatchResponseSchema.Type;

// =============================================================================
// Validation Effects
// =============================================================================

/**
 * Validate an RPC path using Effect Schema.
 */
export const validateRpcPath = (
  path: unknown,
): Effect.Effect<string, RpcEffectError> =>
  Schema.decodeUnknown(RpcPathSchema)(path).pipe(
    Effect.mapError((error) =>
      createValidationError("path", [
        {
          path: [],
          message: `Invalid RPC path: ${error.message}`,
          code: "INVALID_PATH",
        },
      ]),
    ),
  );

/**
 * Validate an RPC request using Effect Schema.
 */
export const validateRpcRequest = (
  request: unknown,
): Effect.Effect<RpcRequest, RpcEffectError> =>
  Schema.decodeUnknown(RpcRequestSchema)(request).pipe(
    Effect.mapError((error) =>
      createValidationError("request", [
        {
          path: [],
          message: `Invalid RPC request: ${error.message}`,
          code: "INVALID_REQUEST",
        },
      ]),
    ),
  );

/**
 * Validate an RPC response using Effect Schema.
 */
export const validateRpcResponse = (
  response: unknown,
): Effect.Effect<RpcResponse, RpcEffectError> =>
  Schema.decodeUnknown(RpcResponseSchema)(response).pipe(
    Effect.mapError((error) =>
      createValidationError("response", [
        {
          path: [],
          message: `Invalid RPC response: ${error.message}`,
          code: "INVALID_RESPONSE",
        },
      ]),
    ),
  );

/**
 * Decode an RPC request from unknown input.
 */
export const decodeRpcRequest = Schema.decodeUnknown(RpcRequestSchema);

/**
 * Decode an RPC response from unknown input.
 */
export const decodeRpcResponse = Schema.decodeUnknown(RpcResponseSchema);

/**
 * Encode an RPC request to unknown output.
 */
export const encodeRpcRequest = Schema.encodeUnknown(RpcRequestSchema);

/**
 * Encode an RPC response to unknown output.
 */
export const encodeRpcResponse = Schema.encodeUnknown(RpcResponseSchema);

// =============================================================================
// Procedure Schema Definition
// =============================================================================

/**
 * Define a procedure with typed input, output, and error schemas.
 */
export interface ProcedureSchema<
  TInput extends Schema.Schema.Any,
  TOutput extends Schema.Schema.Any,
  TError extends Schema.Schema.Any,
> {
  readonly name: string;
  readonly input: TInput;
  readonly output: TOutput;
  readonly error: TError;
  readonly description?: string;
}

/**
 * Infer the input type from a procedure schema.
 */
export type InferInput<T> =
  T extends ProcedureSchema<infer I, Schema.Schema.Any, Schema.Schema.Any>
    ? Schema.Schema.Type<I>
    : never;

/**
 * Infer the output type from a procedure schema.
 */
export type InferOutput<T> =
  T extends ProcedureSchema<Schema.Schema.Any, infer O, Schema.Schema.Any>
    ? Schema.Schema.Type<O>
    : never;

/**
 * Infer the error type from a procedure schema.
 */
export type InferError<T> =
  T extends ProcedureSchema<Schema.Schema.Any, Schema.Schema.Any, infer E>
    ? Schema.Schema.Type<E>
    : never;

/**
 * Create a procedure schema definition.
 */
export const createProcedureSchema = <
  TInput extends Schema.Schema.Any,
  TOutput extends Schema.Schema.Any,
  TError extends Schema.Schema.Any,
>(
  name: string,
  config: {
    input: TInput;
    output: TOutput;
    error: TError;
    description?: string;
  },
): ProcedureSchema<TInput, TOutput, TError> => ({
  name,
  input: config.input,
  output: config.output,
  error: config.error,
  description: config.description,
});

// =============================================================================
// Schema Validation Service
// =============================================================================

/**
 * Schema validator interface for procedure validation.
 */
export interface SchemaValidator {
  readonly validateInput: <T extends Schema.Schema.Any>(
    schema: T,
    input: unknown,
    path: string,
  ) => Effect.Effect<Schema.Schema.Type<T>, RpcEffectError>;

  readonly validateOutput: <T extends Schema.Schema.Any>(
    schema: T,
    output: unknown,
    path: string,
  ) => Effect.Effect<Schema.Schema.Type<T>, RpcEffectError>;
}

/**
 * Default schema validator implementation.
 */
const defaultSchemaValidator: SchemaValidator = {
  validateInput: <T extends Schema.Schema.Any>(
    schema: T,
    input: unknown,
    path: string,
  ): Effect.Effect<Schema.Schema.Type<T>, RpcEffectError> =>
    Schema.decodeUnknown(schema)(input).pipe(
      Effect.mapError((error) =>
        createValidationError(path, [
          {
            path: [],
            message: `Input validation failed: ${error.message}`,
            code: "INVALID_INPUT",
          },
        ]),
      ),
    ) as Effect.Effect<Schema.Schema.Type<T>, RpcEffectError>,

  validateOutput: <T extends Schema.Schema.Any>(
    schema: T,
    output: unknown,
    path: string,
  ): Effect.Effect<Schema.Schema.Type<T>, RpcEffectError> =>
    Schema.decodeUnknown(schema)(output).pipe(
      Effect.mapError((error) =>
        createValidationError(path, [
          {
            path: [],
            message: `Output validation failed: ${error.message}`,
            code: "INVALID_OUTPUT",
          },
        ]),
      ),
    ) as Effect.Effect<Schema.Schema.Type<T>, RpcEffectError>,
};

/**
 * Schema validation service for RPC procedures.
 */
export class SchemaValidationService extends Context.Tag(
  "SchemaValidationService",
)<SchemaValidationService, SchemaValidator>() {
  static Default = Layer.succeed(
    SchemaValidationService,
    defaultSchemaValidator,
  );

  static layer(validator: SchemaValidator) {
    return Layer.succeed(SchemaValidationService, validator);
  }
}
