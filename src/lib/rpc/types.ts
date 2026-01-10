// =============================================================================
// Core Types
// =============================================================================

/** RPC Error from backend */
export interface RpcError {
  code: string;
  message: string;
  details?: unknown;
}

/** Event with optional metadata */
export interface Event<T> {
  data: T;
  id?: string;
  retry?: number;
}

/** Subscription options */
export interface SubscriptionOptions {
  /** Last event ID for resumption */
  lastEventId?: string;
  /** Abort signal for cancellation */
  signal?: AbortSignal;
}

/** Call options */
export interface CallOptions {
  /** Abort signal for cancellation */
  signal?: AbortSignal;
}

// =============================================================================
// Contract Definition Types
// =============================================================================

/** Procedure types */
export type ProcedureType = 'query' | 'mutation' | 'subscription';

/** Base procedure definition */
export interface ProcedureDef<
  TType extends ProcedureType = ProcedureType,
  TInput = unknown,
  TOutput = unknown,
> {
  type: TType;
  input: TInput;
  output: TOutput;
}

/** Query procedure */
export interface QueryDef<TInput = unknown, TOutput = unknown> 
  extends ProcedureDef<'query', TInput, TOutput> {}

/** Mutation procedure */
export interface MutationDef<TInput = unknown, TOutput = unknown> 
  extends ProcedureDef<'mutation', TInput, TOutput> {}

/** Subscription procedure (event iterator) */
export interface SubscriptionDef<TInput = unknown, TOutput = unknown> 
  extends ProcedureDef<'subscription', TInput, TOutput> {}

/** Contract router - nested structure of procedures */
export type ContractRouter = {
  [key: string]: ProcedureDef | ContractRouter;
};

// =============================================================================
// Type Inference Utilities
// =============================================================================

/** Extract input type from procedure */
export type InferInput<T> = T extends ProcedureDef<any, infer I, any> ? I : never;

/** Extract output type from procedure */
export type InferOutput<T> = T extends ProcedureDef<any, any, infer O> ? O : never;

/** Extract procedure type */
export type InferProcedureType<T> = T extends ProcedureDef<infer P, any, any> ? P : never;

/** Check if procedure is a subscription */
export type IsSubscription<T> = T extends SubscriptionDef<any, any> ? true : false;

// =============================================================================
// Client Type Generation
// =============================================================================

/** Event iterator return type */
export interface EventIterator<T> extends AsyncIterable<T> {
  /** Stop the stream manually */
  return(): Promise<void>;
  /** Get the underlying async iterator */
  [Symbol.asyncIterator](): AsyncIterator<T>;
}

/** Convert procedure def to client method */
export type ProcedureClient<T extends ProcedureDef> = 
  T extends SubscriptionDef<infer I, infer O>
    ? I extends void | undefined | never
      ? (options?: SubscriptionOptions) => Promise<EventIterator<O>>
      : (input: I, options?: SubscriptionOptions) => Promise<EventIterator<O>>
    : T extends QueryDef<infer I, infer O> | MutationDef<infer I, infer O>
      ? I extends void | undefined | never
        ? (options?: CallOptions) => Promise<O>
        : (input: I, options?: CallOptions) => Promise<O>
      : never;

/** Convert contract router to client type */
export type RouterClient<T extends ContractRouter> = {
  [K in keyof T]: T[K] extends ProcedureDef
    ? ProcedureClient<T[K]>
    : T[K] extends ContractRouter
      ? RouterClient<T[K]>
      : never;
};

// =============================================================================
// Contract Builder Helpers (for defining contracts)
// =============================================================================

/** Define a query procedure */
export function query<TInput = void, TOutput = void>(): QueryDef<TInput, TOutput> {
  return { type: 'query', input: undefined as TInput, output: undefined as TOutput };
}

/** Define a mutation procedure */
export function mutation<TInput = void, TOutput = void>(): MutationDef<TInput, TOutput> {
  return { type: 'mutation', input: undefined as TInput, output: undefined as TOutput };
}

/** Define a subscription procedure */
export function subscription<TInput = void, TOutput = void>(): SubscriptionDef<TInput, TOutput> {
  return { type: 'subscription', input: undefined as TInput, output: undefined as TOutput };
}
