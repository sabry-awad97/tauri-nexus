/**
 * Type-safe client for Tauri commands
 */

import { invoke } from '@tauri-apps/api/core';
import type { RouterDef, ProcedureDef } from './types';
import { TauriRPCError } from './types';

type Callable<TInput, TOutput> = TInput extends void
  ? () => Promise<TOutput>
  : (input: TInput) => Promise<TOutput>;

type ClientRouter<T extends RouterDef> = {
  [K in keyof T]: T[K] extends ProcedureDef<infer I, infer O>
    ? Callable<I, O>
    : T[K] extends RouterDef
      ? ClientRouter<T[K]>
      : never;
};

export interface ClientConfig {
  onError?: (error: TauriRPCError) => void;
}

export function createClient<T extends RouterDef>(
  routerDef: T,
  config: ClientConfig = {}
): ClientRouter<T> {
  const { onError } = config;

  function createProxy(target: RouterDef): any {
    return new Proxy({} as any, {
      get(_, prop: string) {
        const value = target[prop];
        if (!value) return undefined;

        if ('_command' in value) {
          const proc = value as ProcedureDef<any, any>;
          return async (input?: any) => {
            try {
              return await invoke(proc._command, input ?? {});
            } catch (error) {
              const rpcError = new TauriRPCError(
                error instanceof Error ? error.message : String(error),
                'INVOKE_ERROR',
                error
              );
              onError?.(rpcError);
              throw rpcError;
            }
          };
        }

        return createProxy(value as RouterDef);
      },
    });
  }

  return createProxy(routerDef);
}
