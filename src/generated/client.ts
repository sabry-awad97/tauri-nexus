// =============================================================================
// RPC Client
// =============================================================================
// Low-level RPC client for making calls to the Tauri backend.

import { invoke } from "@tauri-apps/api/core";
import type { RpcError } from "./types";

// -----------------------------------------------------------------------------
// Client Configuration
// -----------------------------------------------------------------------------

export interface RpcClientConfig {
  /** Called before each request */
  onRequest?: (path: string, input: unknown) => void;
  /** Called after successful response */
  onResponse?: (path: string, data: unknown) => void;
  /** Called on error */
  onError?: (path: string, error: RpcError) => void;
}

let config: RpcClientConfig = {};

/** Configure the RPC client */
export function configure(options: RpcClientConfig): void {
  config = { ...config, ...options };
}

// -----------------------------------------------------------------------------
// Core Functions
// -----------------------------------------------------------------------------

/** Parse RPC error from string */
function parseError(error: unknown): RpcError {
  if (typeof error === "string") {
    try {
      return JSON.parse(error) as RpcError;
    } catch {
      return { code: "UNKNOWN", message: error };
    }
  }
  if (error instanceof Error) {
    return { code: "UNKNOWN", message: error.message };
  }
  return { code: "UNKNOWN", message: String(error) };
}

/** Make an RPC call */
export async function call<T>(path: string, input: unknown = null): Promise<T> {
  config.onRequest?.(path, input);

  try {
    const result = await invoke<T>("plugin:rpc|rpc_call", { path, input });
    config.onResponse?.(path, result);
    return result;
  } catch (error) {
    const rpcError = parseError(error);
    config.onError?.(path, rpcError);
    throw rpcError;
  }
}

/** Get list of available procedures */
export async function getProcedures(): Promise<string[]> {
  return invoke<string[]>("plugin:rpc|rpc_procedures");
}

// -----------------------------------------------------------------------------
// Error Utilities
// -----------------------------------------------------------------------------

/** Check if error is an RPC error */
export function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error
  );
}

/** Check if error has a specific code */
export function hasErrorCode(error: unknown, code: string): boolean {
  return isRpcError(error) && error.code === code;
}
