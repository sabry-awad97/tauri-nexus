// =============================================================================
// ApiDocsProvider - Context Provider for API Documentation
// =============================================================================

import {
  createContext,
  useContext,
  useState,
  useMemo,
  useCallback,
  type ReactNode,
} from "react";
import { useRouterSchema } from "../useRouterSchema";
import { groupProcedures, filterProcedures } from "../utils";
import type {
  ProcedureType,
  FilterState,
  ProcedureEntry,
  ProcedureGroup,
  RouterSchema,
} from "../types";

export interface ApiDocsContextValue {
  // Schema data
  schema: RouterSchema | undefined;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;

  // Filter state
  filter: FilterState;
  setSearch: (search: string) => void;
  setTypeFilter: (type: ProcedureType | "all") => void;
  clearFilters: () => void;

  // Filtered data
  procedures: ProcedureEntry[];
  groups: ProcedureGroup[];
  totalCount: number;
  filteredCount: number;

  // Expansion state
  expandedPaths: Set<string>;
  toggleProcedure: (path: string) => void;
  expandAll: () => void;
  collapseAll: () => void;
  isExpanded: (path: string) => boolean;
}

const ApiDocsContext = createContext<ApiDocsContextValue | null>(null);

export interface ApiDocsProviderProps {
  children: ReactNode;
  /** Initial filter type */
  initialFilter?: ProcedureType | "all";
  /** Initial search query */
  initialSearch?: string;
}

export function ApiDocsProvider({
  children,
  initialFilter = "all",
  initialSearch = "",
}: ApiDocsProviderProps) {
  const { data: schema, isLoading, error, refetch } = useRouterSchema();

  // Filter state
  const [filter, setFilter] = useState<FilterState>({
    search: initialSearch,
    typeFilter: initialFilter,
  });

  // Expansion state
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

  // Computed filtered data
  const { procedures, groups, totalCount, filteredCount } = useMemo(() => {
    if (!schema?.procedures) {
      return { procedures: [], groups: [], totalCount: 0, filteredCount: 0 };
    }

    const result = filterProcedures(schema.procedures, filter);
    const filteredRecord: Record<string, (typeof schema.procedures)[string]> =
      {};
    for (const proc of result.procedures) {
      filteredRecord[proc.path] = proc.schema;
    }

    return {
      procedures: result.procedures,
      groups: groupProcedures(filteredRecord),
      totalCount: result.totalCount,
      filteredCount: result.count,
    };
  }, [schema?.procedures, filter]);

  // Filter handlers
  const setSearch = useCallback((search: string) => {
    setFilter((prev) => ({ ...prev, search }));
  }, []);

  const setTypeFilter = useCallback((typeFilter: ProcedureType | "all") => {
    setFilter((prev) => ({ ...prev, typeFilter }));
  }, []);

  const clearFilters = useCallback(() => {
    setFilter({ search: "", typeFilter: "all" });
  }, []);

  // Expansion handlers
  const toggleProcedure = useCallback((path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const expandAll = useCallback(() => {
    setExpandedPaths(new Set(procedures.map((p) => p.path)));
  }, [procedures]);

  const collapseAll = useCallback(() => {
    setExpandedPaths(new Set());
  }, []);

  const isExpanded = useCallback(
    (path: string) => expandedPaths.has(path),
    [expandedPaths],
  );

  const value: ApiDocsContextValue = {
    schema,
    isLoading,
    error: error as Error | null,
    refetch,
    filter,
    setSearch,
    setTypeFilter,
    clearFilters,
    procedures,
    groups,
    totalCount,
    filteredCount,
    expandedPaths,
    toggleProcedure,
    expandAll,
    collapseAll,
    isExpanded,
  };

  return (
    <ApiDocsContext.Provider value={value}>{children}</ApiDocsContext.Provider>
  );
}

export function useApiDocsContext(): ApiDocsContextValue {
  const context = useContext(ApiDocsContext);
  if (!context) {
    throw new Error("useApiDocsContext must be used within an ApiDocsProvider");
  }
  return context;
}
