// =============================================================================
// OpenAPI Documentation Utilities
// =============================================================================
// Utility functions for grouping and filtering RPC procedures.

import type {
  ProcedureSchema,
  ProcedureEntry,
  ProcedureGroup,
  ProcedureType,
  FilterState,
  FilterResult,
} from './types';

/**
 * Extract the namespace from a procedure path.
 * For "user.get" returns "user", for "health" returns "" (root level).
 */
export function getNamespace(path: string): string {
  const lastDotIndex = path.lastIndexOf('.');
  return lastDotIndex === -1 ? '' : path.substring(0, lastDotIndex);
}

/**
 * Get the procedure name without namespace.
 * For "user.get" returns "get", for "health" returns "health".
 */
export function getProcedureName(path: string): string {
  const lastDotIndex = path.lastIndexOf('.');
  return lastDotIndex === -1 ? path : path.substring(lastDotIndex + 1);
}

/**
 * Group procedures by their namespace prefix.
 * 
 * @param procedures - Record of procedure path to schema
 * @returns Array of procedure groups sorted by namespace
 * 
 * @example
 * ```typescript
 * const groups = groupProcedures({
 *   'health': { ... },
 *   'user.get': { ... },
 *   'user.create': { ... },
 * });
 * // Returns:
 * // [
 * //   { namespace: '', procedures: [{ path: 'health', ... }] },
 * //   { namespace: 'user', procedures: [{ path: 'user.get', ... }, { path: 'user.create', ... }] }
 * // ]
 * ```
 */
export function groupProcedures(
  procedures: Record<string, ProcedureSchema>
): ProcedureGroup[] {
  const groupMap = new Map<string, ProcedureEntry[]>();

  // Group procedures by namespace
  for (const [path, schema] of Object.entries(procedures)) {
    const namespace = getNamespace(path);
    const entry: ProcedureEntry = { path, schema };

    if (!groupMap.has(namespace)) {
      groupMap.set(namespace, []);
    }
    groupMap.get(namespace)!.push(entry);
  }

  // Convert to array and sort
  const groups: ProcedureGroup[] = [];
  for (const [namespace, procs] of groupMap) {
    // Sort procedures within group by path
    procs.sort((a, b) => a.path.localeCompare(b.path));
    groups.push({ namespace, procedures: procs });
  }

  // Sort groups: root level first, then alphabetically
  groups.sort((a, b) => {
    if (a.namespace === '' && b.namespace !== '') return -1;
    if (a.namespace !== '' && b.namespace === '') return 1;
    return a.namespace.localeCompare(b.namespace);
  });

  return groups;
}

/**
 * Check if a procedure matches the search query.
 * Searches in path and description (case-insensitive).
 */
function matchesSearch(entry: ProcedureEntry, search: string): boolean {
  if (!search) return true;
  
  const lowerSearch = search.toLowerCase();
  const pathMatch = entry.path.toLowerCase().includes(lowerSearch);
  const descMatch = entry.schema.description?.toLowerCase().includes(lowerSearch) ?? false;
  
  return pathMatch || descMatch;
}

/**
 * Check if a procedure matches the type filter.
 */
function matchesType(entry: ProcedureEntry, typeFilter: ProcedureType | 'all'): boolean {
  if (typeFilter === 'all') return true;
  return entry.schema.procedure_type === typeFilter;
}

/**
 * Filter procedures by search query and type.
 * 
 * @param procedures - Record of procedure path to schema
 * @param filter - Filter state with search and typeFilter
 * @returns Filtered procedures with counts
 * 
 * @example
 * ```typescript
 * const result = filterProcedures(procedures, {
 *   search: 'user',
 *   typeFilter: 'query'
 * });
 * // Returns procedures matching both criteria
 * ```
 */
export function filterProcedures(
  procedures: Record<string, ProcedureSchema>,
  filter: FilterState
): FilterResult {
  const entries = Object.entries(procedures).map(([path, schema]) => ({
    path,
    schema,
  }));

  const totalCount = entries.length;
  
  const filtered = entries.filter(
    (entry) =>
      matchesSearch(entry, filter.search) && matchesType(entry, filter.typeFilter)
  );

  return {
    procedures: filtered,
    count: filtered.length,
    totalCount,
  };
}

/**
 * Get display label for a procedure type.
 */
export function getTypeLabel(type: ProcedureType): string {
  switch (type) {
    case 'query':
      return 'Query';
    case 'mutation':
      return 'Mutation';
    case 'subscription':
      return 'Subscription';
  }
}

/**
 * Get CSS class for a procedure type badge.
 */
export function getTypeBadgeClass(type: ProcedureType): string {
  switch (type) {
    case 'query':
      return 'badge-query';
    case 'mutation':
      return 'badge-mutation';
    case 'subscription':
      return 'badge-subscription';
  }
}
