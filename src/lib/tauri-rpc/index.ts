/**
 * Tauri RPC - Type-safe communication for Tauri v2
 */

export { procedure, router, ProcedureBuilder } from './builder';
export { createClient, type ClientConfig } from './client';
export { createReactClient, type UseQueryResult, type UseMutationResult } from './react';
export { TauriRPCError, type ProcedureType, type ProcedureDef, type RouterDef, type InferInput, type InferOutput } from './types';


