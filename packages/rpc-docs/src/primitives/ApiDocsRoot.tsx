// =============================================================================
// ApiDocsRoot - Root Layout Components
// =============================================================================

import { type ReactNode, type HTMLAttributes, forwardRef } from "react";
import { useApiDocsContext } from "./ApiDocsProvider";

export interface ApiDocsRootProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/** Root container for API documentation */
export const ApiDocsRoot = forwardRef<HTMLDivElement, ApiDocsRootProps>(
  ({ children, className, ...props }, ref) => {
    return (
      <div ref={ref} className={className} data-testid="api-docs" {...props}>
        {children}
      </div>
    );
  },
);
ApiDocsRoot.displayName = "ApiDocsRoot";

/** Header section container */
export const ApiDocsHeader = forwardRef<
  HTMLElement,
  HTMLAttributes<HTMLElement>
>(({ children, className, ...props }, ref) => {
  return (
    <header
      ref={ref}
      className={className}
      data-testid="api-docs-header"
      {...props}
    >
      {children}
    </header>
  );
});
ApiDocsHeader.displayName = "ApiDocsHeader";

interface ApiDocsTitleProps extends HTMLAttributes<HTMLHeadingElement> {
  children?: ReactNode;
  /** Custom title, defaults to schema name or "API Documentation" */
  title?: string;
}

/** Title component - renders schema name or custom title */
export const ApiDocsTitle = forwardRef<HTMLHeadingElement, ApiDocsTitleProps>(
  ({ children, title, className, ...props }, ref) => {
    const { schema } = useApiDocsContext();
    const displayTitle =
      children ?? title ?? schema?.name ?? "API Documentation";

    return (
      <h2
        ref={ref}
        className={className}
        data-testid="api-docs-title"
        {...props}
      >
        {displayTitle}
      </h2>
    );
  },
);
ApiDocsTitle.displayName = "ApiDocsTitle";

interface ApiDocsDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
  children?: ReactNode;
  /** Custom description */
  description?: string;
}

/** Description component */
export const ApiDocsDescription = forwardRef<
  HTMLParagraphElement,
  ApiDocsDescriptionProps
>(({ children, description, className, ...props }, ref) => {
  const { schema } = useApiDocsContext();
  const displayDescription = children ?? description ?? schema?.description;

  if (!displayDescription) return null;

  return (
    <p
      ref={ref}
      className={className}
      data-testid="api-docs-description"
      {...props}
    >
      {displayDescription}
    </p>
  );
});
ApiDocsDescription.displayName = "ApiDocsDescription";

/** Version badge component */
export const ApiDocsVersion = forwardRef<
  HTMLSpanElement,
  HTMLAttributes<HTMLSpanElement>
>(({ children, className, ...props }, ref) => {
  const { schema } = useApiDocsContext();

  if (!schema?.version && !children) return null;

  return (
    <span
      ref={ref}
      className={className}
      data-testid="api-docs-version"
      {...props}
    >
      {children ?? `v${schema?.version}`}
    </span>
  );
});
ApiDocsVersion.displayName = "ApiDocsVersion";

interface ApiDocsActionsProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
  /** Render prop for custom actions */
  render?: (props: {
    expandAll: () => void;
    collapseAll: () => void;
    canExpand: boolean;
    canCollapse: boolean;
  }) => ReactNode;
}

/** Actions container with expand/collapse helpers */
export const ApiDocsActions = forwardRef<HTMLDivElement, ApiDocsActionsProps>(
  ({ children, render, className, ...props }, ref) => {
    const { expandAll, collapseAll, filteredCount, expandedPaths } =
      useApiDocsContext();

    const actionProps = {
      expandAll,
      collapseAll,
      canExpand: filteredCount > 0,
      canCollapse: expandedPaths.size > 0,
    };

    return (
      <div
        ref={ref}
        className={className}
        data-testid="api-docs-actions"
        {...props}
      >
        {render ? render(actionProps) : children}
      </div>
    );
  },
);
ApiDocsActions.displayName = "ApiDocsActions";

/** Main content container */
export const ApiDocsContent = forwardRef<
  HTMLDivElement,
  HTMLAttributes<HTMLDivElement>
>(({ children, className, ...props }, ref) => {
  return (
    <div
      ref={ref}
      className={className}
      data-testid="api-docs-content"
      {...props}
    >
      {children}
    </div>
  );
});
ApiDocsContent.displayName = "ApiDocsContent";

interface ApiDocsEmptyProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
  /** Render prop with clear filters action */
  render?: (props: {
    clearFilters: () => void;
    hasSearch: boolean;
  }) => ReactNode;
}

/** Empty state component - shown when no procedures match filters */
export const ApiDocsEmpty = forwardRef<HTMLDivElement, ApiDocsEmptyProps>(
  ({ children, render, className, ...props }, ref) => {
    const { filteredCount, filter, clearFilters } = useApiDocsContext();

    if (filteredCount > 0) return null;

    const emptyProps = {
      clearFilters,
      hasSearch: filter.search.length > 0,
    };

    return (
      <div
        ref={ref}
        className={className}
        data-testid="api-docs-empty"
        {...props}
      >
        {render
          ? render(emptyProps)
          : (children ?? <p>No procedures match your filters.</p>)}
      </div>
    );
  },
);
ApiDocsEmpty.displayName = "ApiDocsEmpty";

/** Loading state component */
export const ApiDocsLoading = forwardRef<
  HTMLDivElement,
  HTMLAttributes<HTMLDivElement>
>(({ children, className, ...props }, ref) => {
  const { isLoading } = useApiDocsContext();

  if (!isLoading) return null;

  return (
    <div
      ref={ref}
      className={className}
      data-testid="api-docs-loading"
      {...props}
    >
      {children ?? <p>Loading API documentation...</p>}
    </div>
  );
});
ApiDocsLoading.displayName = "ApiDocsLoading";

interface ApiDocsErrorProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
  /** Render prop with error and refetch */
  render?: (props: { error: Error; refetch: () => void }) => ReactNode;
}

/** Error state component */
export const ApiDocsError = forwardRef<HTMLDivElement, ApiDocsErrorProps>(
  ({ children, render, className, ...props }, ref) => {
    const { error, refetch } = useApiDocsContext();

    if (!error) return null;

    const errorProps = { error, refetch };

    return (
      <div
        ref={ref}
        className={className}
        data-testid="api-docs-error"
        {...props}
      >
        {render
          ? render(errorProps)
          : (children ?? (
              <p>Failed to load API documentation: {error.message}</p>
            ))}
      </div>
    );
  },
);
ApiDocsError.displayName = "ApiDocsError";
