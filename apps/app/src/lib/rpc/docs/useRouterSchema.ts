// =============================================================================
// useRouterSchema Hook
// =============================================================================
// Hook for fetching router schema from the backend.

import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { RouterSchema, ProcedureSchema } from "./types";

/**
 * Query key for router schema.
 */
export const ROUTER_SCHEMA_KEY = ["rpc", "schema"] as const;

/**
 * Infer procedure type from path naming conventions.
 * - Paths containing 'stream' or ending with subscription-like names → subscription
 * - Paths containing 'create', 'update', 'delete', 'set', 'add', 'remove' → mutation
 * - Everything else → query
 */
function inferProcedureType(
  path: string,
): "query" | "mutation" | "subscription" {
  const lowerPath = path.toLowerCase();

  // Check for subscription patterns
  if (
    lowerPath.includes("stream") ||
    lowerPath.includes("subscribe") ||
    lowerPath.includes("watch")
  ) {
    return "subscription";
  }

  // Check for mutation patterns
  const mutationPatterns = [
    "create",
    "update",
    "delete",
    "set",
    "add",
    "remove",
    "send",
    "post",
    "put",
  ];
  for (const pattern of mutationPatterns) {
    if (lowerPath.includes(pattern)) {
      return "mutation";
    }
  }

  return "query";
}

/**
 * Fetch router schema from the backend.
 * The backend returns procedure names, which we convert to a RouterSchema.
 */
async function fetchRouterSchema(): Promise<RouterSchema> {
  // Call the plugin command to get procedure names
  const procedureNames = await invoke<string[]>("plugin:rpc|rpc_procedures");

  // Build schema from procedure names
  const procedures: Record<string, ProcedureSchema> = {};

  for (const path of procedureNames) {
    const procedureType = inferProcedureType(path);

    procedures[path] = {
      procedure_type: procedureType,
      description: undefined,
      input: undefined,
      output: undefined,
      deprecated: false,
      tags: [],
      metadata: undefined,
    };
  }

  return {
    version: "1.0.0",
    name: "RPC API",
    description: "Available RPC procedures",
    procedures,
    metadata: undefined,
  };
}

/**
 * Hook for fetching and caching the router schema.
 *
 * @example
 * ```tsx
 * function ApiDocs() {
 *   const { data, isLoading, error, refetch } = useRouterSchema();
 *
 *   if (isLoading) return <div>Loading...</div>;
 *   if (error) return <div>Error: {error.message}</div>;
 *
 *   return <div>{Object.keys(data.procedures).length} procedures</div>;
 * }
 * ```
 */
export function useRouterSchema() {
  return useQuery({
    queryKey: ROUTER_SCHEMA_KEY,
    queryFn: fetchRouterSchema,
    staleTime: 1000 * 60 * 5, // 5 minutes
    retry: 2,
  });
}

export default useRouterSchema;
