// =============================================================================
// ApiDocsFilter - Filter Components
// =============================================================================

import {
  type InputHTMLAttributes,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  forwardRef,
  useState,
  useEffect,
  useCallback,
} from "react";
import { useApiDocsContext } from "./ApiDocsProvider";
import type { ProcedureType } from "../types";

export interface ApiDocsSearchProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "onChange" | "value"
> {
  /** Debounce delay in ms (default: 200) */
  debounceMs?: number;
  /** Called when search value changes (after debounce) */
  onValueChange?: (value: string) => void;
}

/** Search input component with debouncing */
export const ApiDocsSearch = forwardRef<HTMLInputElement, ApiDocsSearchProps>(
  ({ debounceMs = 200, onValueChange, className, ...props }, ref) => {
    const { filter, setSearch } = useApiDocsContext();
    const [localValue, setLocalValue] = useState(filter.search);

    // Sync with external changes
    useEffect(() => {
      setLocalValue(filter.search);
    }, [filter.search]);

    // Debounced update
    useEffect(() => {
      const timer = setTimeout(() => {
        if (localValue !== filter.search) {
          setSearch(localValue);
          onValueChange?.(localValue);
        }
      }, debounceMs);

      return () => clearTimeout(timer);
    }, [localValue, filter.search, setSearch, debounceMs, onValueChange]);

    const handleChange = useCallback(
      (e: React.ChangeEvent<HTMLInputElement>) => {
        setLocalValue(e.target.value);
      },
      [],
    );

    return (
      <input
        ref={ref}
        type="text"
        value={localValue}
        onChange={handleChange}
        placeholder="Search procedures..."
        aria-label="Search procedures"
        className={className}
        data-testid="api-docs-search"
        {...props}
      />
    );
  },
);
ApiDocsSearch.displayName = "ApiDocsSearch";

export interface ApiDocsTypeFilterProps extends HTMLAttributes<HTMLDivElement> {
  /** Render prop for custom filter buttons */
  render?: (props: {
    filters: Array<{
      value: ProcedureType | "all";
      label: string;
      isActive: boolean;
    }>;
    onSelect: (value: ProcedureType | "all") => void;
  }) => React.ReactNode;
  /** Custom filter options */
  options?: Array<{ value: ProcedureType | "all"; label: string }>;
}

const DEFAULT_FILTERS: Array<{ value: ProcedureType | "all"; label: string }> =
  [
    { value: "all", label: "All" },
    { value: "query", label: "Queries" },
    { value: "mutation", label: "Mutations" },
    { value: "subscription", label: "Subscriptions" },
  ];

/** Type filter component */
export const ApiDocsTypeFilter = forwardRef<
  HTMLDivElement,
  ApiDocsTypeFilterProps
>(
  (
    { render, options = DEFAULT_FILTERS, className, children, ...props },
    ref,
  ) => {
    const { filter, setTypeFilter } = useApiDocsContext();

    const filters = options.map((opt) => ({
      ...opt,
      isActive: filter.typeFilter === opt.value,
    }));

    if (render) {
      return (
        <div
          ref={ref}
          className={className}
          data-testid="api-docs-type-filter"
          {...props}
        >
          {render({ filters, onSelect: setTypeFilter })}
        </div>
      );
    }

    return (
      <div
        ref={ref}
        className={className}
        data-testid="api-docs-type-filter"
        {...props}
      >
        {children}
      </div>
    );
  },
);
ApiDocsTypeFilter.displayName = "ApiDocsTypeFilter";

export interface TypeFilterButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  value: ProcedureType | "all";
  /** Active class name to apply when this filter is selected */
  activeClassName?: string;
}

/** Individual type filter button */
export const ApiDocsTypeFilterButton = forwardRef<
  HTMLButtonElement,
  TypeFilterButtonProps
>(({ value, activeClassName, className, children, ...props }, ref) => {
  const { filter, setTypeFilter } = useApiDocsContext();
  const isActive = filter.typeFilter === value;

  return (
    <button
      ref={ref}
      type="button"
      onClick={() => setTypeFilter(value)}
      aria-pressed={isActive}
      className={`${className ?? ""} ${isActive && activeClassName ? activeClassName : ""}`.trim()}
      data-testid={`api-docs-filter-${value}`}
      data-active={isActive}
      {...props}
    >
      {children}
    </button>
  );
});
ApiDocsTypeFilterButton.displayName = "ApiDocsTypeFilterButton";

export interface ApiDocsCountProps extends HTMLAttributes<HTMLSpanElement> {
  /** Render prop for custom count display */
  render?: (props: { filtered: number; total: number }) => React.ReactNode;
}

/** Procedure count display */
export const ApiDocsCount = forwardRef<HTMLSpanElement, ApiDocsCountProps>(
  ({ render, className, children, ...props }, ref) => {
    const { filteredCount, totalCount } = useApiDocsContext();

    if (render) {
      return (
        <span
          ref={ref}
          className={className}
          data-testid="api-docs-count"
          {...props}
        >
          {render({ filtered: filteredCount, total: totalCount })}
        </span>
      );
    }

    return (
      <span
        ref={ref}
        className={className}
        data-testid="api-docs-count"
        {...props}
      >
        {children ?? `${filteredCount} / ${totalCount} procedures`}
      </span>
    );
  },
);
ApiDocsCount.displayName = "ApiDocsCount";
