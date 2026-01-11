// =============================================================================
// FilterBar Component
// =============================================================================
// Provides search and filter controls for the API documentation.

import { useState, useEffect, useCallback, JSX } from "react";
import type { ProcedureType } from "./types";

export interface FilterBarProps {
  /** Current search query */
  search: string;
  /** Callback when search changes */
  onSearchChange: (value: string) => void;
  /** Current procedure type filter */
  typeFilter: ProcedureType | "all";
  /** Callback when type filter changes */
  onTypeFilterChange: (type: ProcedureType | "all") => void;
  /** Total procedure count */
  totalCount: number;
  /** Filtered procedure count */
  filteredCount: number;
  /** Debounce delay in ms (default: 200) */
  debounceMs?: number;
}

const TYPE_FILTERS: Array<{ value: ProcedureType | "all"; label: string }> = [
  { value: "all", label: "All" },
  { value: "query", label: "Queries" },
  { value: "mutation", label: "Mutations" },
  { value: "subscription", label: "Subscriptions" },
];

/**
 * FilterBar component for filtering procedures.
 */
export function FilterBar({
  search,
  onSearchChange,
  typeFilter,
  onTypeFilterChange,
  totalCount,
  filteredCount,
  debounceMs = 200,
}: FilterBarProps): JSX.Element {
  const [localSearch, setLocalSearch] = useState(search);

  // Sync local search with prop when prop changes externally
  useEffect(() => {
    setLocalSearch(search);
  }, [search]);

  // Debounced search update
  useEffect(() => {
    const timer = setTimeout(() => {
      if (localSearch !== search) {
        onSearchChange(localSearch);
      }
    }, debounceMs);

    return () => clearTimeout(timer);
  }, [localSearch, search, onSearchChange, debounceMs]);

  const handleSearchChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setLocalSearch(e.target.value);
    },
    [],
  );

  const handleClearSearch = useCallback(() => {
    setLocalSearch("");
    onSearchChange("");
  }, [onSearchChange]);

  return (
    <div className="filter-bar" data-testid="filter-bar">
      <div className="filter-search">
        <input
          type="text"
          className="filter-search-input"
          placeholder="Search procedures..."
          value={localSearch}
          onChange={handleSearchChange}
          aria-label="Search procedures"
          data-testid="search-input"
        />
        {localSearch && (
          <button
            type="button"
            className="filter-search-clear"
            onClick={handleClearSearch}
            aria-label="Clear search"
            data-testid="clear-search"
          >
            ✕
          </button>
        )}
      </div>

      <div className="filter-types" data-testid="type-filters">
        {TYPE_FILTERS.map(({ value, label }) => (
          <button
            key={value}
            type="button"
            className={`filter-type-btn ${typeFilter === value ? "active" : ""}`}
            onClick={() => onTypeFilterChange(value)}
            aria-pressed={typeFilter === value}
            data-testid={`filter-${value}`}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="filter-count" data-testid="filter-count">
        <span className="filter-count-value">{filteredCount}</span>
        <span className="filter-count-separator">/</span>
        <span className="filter-count-total">{totalCount}</span>
        <span className="filter-count-label">procedures</span>
      </div>
    </div>
  );
}

export default FilterBar;
