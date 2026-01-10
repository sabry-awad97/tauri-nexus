/**
 * Core type definitions for Tauri RPC
 */

export type ProcedureType = 'query' | 'mutation';

export interface ProcedureDef<TInput = unknown, TOutput = unknown> {
  _type: ProcedureType;
  _input: TInput;
  _output: TOutput;
  _command: string;
}

export type RouterDef = {
  [key: string]: ProcedureDef<any, any> | RouterDef;
};

export type InferInput<T> = T extends ProcedureDef<infer I, any> ? I : never;
export type InferOutput<T> = T extends ProcedureDef<any, infer O> ? O : never;

export class TauriRPCError extends Error {
  constructor(
    message: string,
    public code: string,
    public cause?: unknown
  ) {
    super(message);
    this.name = 'TauriRPCError';
  }
}
