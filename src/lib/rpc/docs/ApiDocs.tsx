// =============================================================================
// ApiDocs Component
// =============================================================================
// Main API documentation component that displays router schema.

import './styles.css';
import { useState, useMemo, useCallback } from 'react';
import { useRouterSchema } from './useRouterSchema';
import { FilterBar } from './FilterBar';
import { ProcedureCard } from './ProcedureCard';
import { groupProcedures, filterProcedures } from './utils';
import type { ProcedureType, FilterState } from './types';

export interface ApiDocsProps {
  /** Custom title for the documentation */
  title?: string;
  /** Custom description */
  description?: string;
  /** Initial filter state */
  initialFilter?: ProcedureType | 'all';
  /** Whether to show the header (default: true) */
  showHeader?: boolean;
  /** Custom class name for styling */
  className?: string;
}

/**
 * ApiDocs component for displaying API documentation.
 * 
 * @example
 * ```tsx
 * // Basic usage
 * <ApiDocs />
 * 
 * // With customization
 * <ApiDocs
 *   title="My API"
 *   description="API documentation for my application"
 *   initialFilter="query"
 * />
 * ```
 */
export function ApiDocs({
  title = 'API Documentation',
  description = 'Browse and explore available RPC procedures',
  initialFilter = 'all',
  showHeader = true,
  className = '',
}: ApiDocsProps): JSX.Element {
  const { data: schema, isLoading, error, refetch } = useRouterSchema();
  
  // Filter state
  const [filter, setFilter] = useState<FilterState>({
    search: '',
    typeFilter: initialFilter,
  });
  
  // Expanded procedures state
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

  // Filter and group procedures
  const { filteredProcedures, groups, totalCount, filteredCount } = useMemo(() => {
    if (!schema?.procedures) {
      return { filteredProcedures: [], groups: [], totalCount: 0, filteredCount: 0 };
    }

    const result = filterProcedures(schema.procedures, filter);
    const filteredRecord: Record<string, typeof schema.procedures[string]> = {};
    for (const proc of result.procedures) {
      filteredRecord[proc.path] = proc.schema;
    }
    
    return {
      filteredProcedures: result.procedures,
      groups: groupProcedures(filteredRecord),
      totalCount: result.totalCount,
      filteredCount: result.count,
    };
  }, [schema?.procedures, filter]);

  // Handlers
  const handleSearchChange = useCallback((search: string) => {
    setFilter((prev) => ({ ...prev, search }));
  }, []);

  const handleTypeFilterChange = useCallback((typeFilter: ProcedureType | 'all') => {
    setFilter((prev) => ({ ...prev, typeFilter }));
  }, []);

  const handleToggleProcedure = useCallback((path: string) => {
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

  const handleExpandAll = useCallback(() => {
    setExpandedPaths(new Set(filteredProcedures.map((p) => p.path)));
  }, [filteredProcedures]);

  const handleCollapseAll = useCallback(() => {
    setExpandedPaths(new Set());
  }, []);

  // Loading state
  if (isLoading) {
    return (
      <div className={`api-docs api-docs-loading ${className}`} data-testid="api-docs-loading">
        <div className="api-docs-spinner" />
        <p>Loading API documentation...</p>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className={`api-docs api-docs-error ${className}`} data-testid="api-docs-error">
        <div className="api-docs-error-icon">⚠️</div>
        <h3>Failed to load API documentation</h3>
        <p>{error instanceof Error ? error.message : 'Unknown error'}</p>
        <button
          type="button"
          className="api-docs-retry-btn"
          onClick={() => refetch()}
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className={`api-docs ${className}`} data-testid="api-docs">
      {showHeader && (
        <header className="api-docs-header" data-testid="api-docs-header">
          <div className="api-docs-header-content">
            <h2 className="api-docs-title" data-testid="api-docs-title">{title}</h2>
            <p className="api-docs-description" data-testid="api-docs-description">{description}</p>
          </div>
          {schema?.version && (
            <span className="api-docs-version">v{schema.version}</span>
          )}
        </header>
      )}

      <FilterBar
        search={filter.search}
        onSearchChange={handleSearchChange}
        typeFilter={filter.typeFilter}
        onTypeFilterChange={handleTypeFilterChange}
        totalCount={totalCount}
        filteredCount={filteredCount}
      />

      <div className="api-docs-actions">
        <button
          type="button"
          className="api-docs-action-btn"
          onClick={handleExpandAll}
          disabled={filteredCount === 0}
        >
          Expand All
        </button>
        <button
          type="button"
          className="api-docs-action-btn"
          onClick={handleCollapseAll}
          disabled={expandedPaths.size === 0}
        >
          Collapse All
        </button>
      </div>

      {filteredCount === 0 ? (
        <div className="api-docs-empty" data-testid="api-docs-empty">
          <p>No procedures match your filters.</p>
          {filter.search && (
            <button
              type="button"
              className="api-docs-clear-btn"
              onClick={() => handleSearchChange('')}
            >
              Clear search
            </button>
          )}
        </div>
      ) : (
        <div className="api-docs-groups" data-testid="api-docs-groups">
          {groups.map((group) => (
            <div key={group.namespace || '__root__'} className="api-docs-group">
              {group.namespace && (
                <h3 className="api-docs-group-title">{group.namespace}</h3>
              )}
              <div className="api-docs-procedures">
                {group.procedures.map((proc) => (
                  <ProcedureCard
                    key={proc.path}
                    path={proc.path}
                    schema={proc.schema}
                    expanded={expandedPaths.has(proc.path)}
                    onToggle={() => handleToggleProcedure(proc.path)}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default ApiDocs;
