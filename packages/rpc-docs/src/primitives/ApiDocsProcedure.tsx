// =============================================================================
// ApiDocsProcedure - Procedure Display Components
// =============================================================================

import {
  type HTMLAttributes,
  type ButtonHTMLAttributes,
  forwardRef,
  createContext,
  useContext,
  type ReactNode,
} from "react";
import { useApiDocsContext } from "./ApiDocsProvider";
import type { ProcedureSchema, ProcedureEntry, ProcedureGroup } from "../types";

// Procedure context for nested components
interface ProcedureContextValue {
  path: string;
  schema: ProcedureSchema;
  expanded: boolean;
  toggle: () => void;
}

const ProcedureContext = createContext<ProcedureContextValue | null>(null);

function useProcedureContext() {
  const context = useContext(ProcedureContext);
  if (!context) {
    throw new Error(
      "Procedure components must be used within ApiDocsProcedureCard",
    );
  }
  return context;
}

export interface ApiDocsProcedureListProps extends HTMLAttributes<HTMLDivElement> {
  /** Render prop for custom list rendering */
  render?: (props: {
    groups: ProcedureGroup[];
    procedures: ProcedureEntry[];
  }) => ReactNode;
}

/** Container for procedure list */
export const ApiDocsProcedureList = forwardRef<
  HTMLDivElement,
  ApiDocsProcedureListProps
>(({ render, className, children, ...props }, ref) => {
  const { groups, procedures } = useApiDocsContext();

  if (render) {
    return (
      <div
        ref={ref}
        className={className}
        data-testid="api-docs-procedure-list"
        {...props}
      >
        {render({ groups, procedures })}
      </div>
    );
  }

  return (
    <div
      ref={ref}
      className={className}
      data-testid="api-docs-procedure-list"
      {...props}
    >
      {children}
    </div>
  );
});
ApiDocsProcedureList.displayName = "ApiDocsProcedureList";

export interface ApiDocsProcedureGroupProps extends HTMLAttributes<HTMLDivElement> {
  /** Group data */
  group: ProcedureGroup;
  /** Render prop for custom group rendering */
  render?: (props: {
    namespace: string;
    procedures: ProcedureEntry[];
  }) => ReactNode;
}

/** Procedure group container */
export const ApiDocsProcedureGroup = forwardRef<
  HTMLDivElement,
  ApiDocsProcedureGroupProps
>(({ group, render, className, children, ...props }, ref) => {
  if (render) {
    return (
      <div
        ref={ref}
        className={className}
        data-testid={`api-docs-group-${group.namespace || "root"}`}
        {...props}
      >
        {render({ namespace: group.namespace, procedures: group.procedures })}
      </div>
    );
  }

  return (
    <div
      ref={ref}
      className={className}
      data-testid={`api-docs-group-${group.namespace || "root"}`}
      {...props}
    >
      {children}
    </div>
  );
});
ApiDocsProcedureGroup.displayName = "ApiDocsProcedureGroup";

export interface ApiDocsProcedureCardProps extends HTMLAttributes<HTMLDivElement> {
  /** Procedure path */
  path: string;
  /** Procedure schema */
  schema: ProcedureSchema;
  /** Override expanded state (controlled mode) */
  expanded?: boolean;
  /** Override toggle handler (controlled mode) */
  onToggle?: () => void;
  /** Data attribute for expanded state */
  expandedClassName?: string;
}

/** Individual procedure card */
export const ApiDocsProcedureCard = forwardRef<
  HTMLDivElement,
  ApiDocsProcedureCardProps
>(
  (
    {
      path,
      schema,
      expanded: controlledExpanded,
      onToggle,
      expandedClassName,
      className,
      children,
      ...props
    },
    ref,
  ) => {
    const { isExpanded, toggleProcedure } = useApiDocsContext();

    const expanded = controlledExpanded ?? isExpanded(path);
    const toggle = onToggle ?? (() => toggleProcedure(path));

    const contextValue: ProcedureContextValue = {
      path,
      schema,
      expanded,
      toggle,
    };

    return (
      <ProcedureContext.Provider value={contextValue}>
        <div
          ref={ref}
          className={`${className ?? ""} ${expanded && expandedClassName ? expandedClassName : ""}`.trim()}
          data-testid={`api-docs-procedure-${path}`}
          data-expanded={expanded}
          {...props}
        >
          {children}
        </div>
      </ProcedureContext.Provider>
    );
  },
);
ApiDocsProcedureCard.displayName = "ApiDocsProcedureCard";

/** Clickable header for procedure card */
export const ApiDocsProcedureHeader = forwardRef<
  HTMLButtonElement,
  ButtonHTMLAttributes<HTMLButtonElement>
>(({ className, children, ...props }, ref) => {
  const { expanded, toggle } = useProcedureContext();

  return (
    <button
      ref={ref}
      type="button"
      onClick={toggle}
      aria-expanded={expanded}
      className={className}
      data-testid="api-docs-procedure-header"
      {...props}
    >
      {children}
    </button>
  );
});
ApiDocsProcedureHeader.displayName = "ApiDocsProcedureHeader";

export interface ApiDocsProcedureBadgeProps extends HTMLAttributes<HTMLSpanElement> {
  /** Custom labels for procedure types */
  labels?: Record<string, string>;
}

/** Procedure type badge */
export const ApiDocsProcedureBadge = forwardRef<
  HTMLSpanElement,
  ApiDocsProcedureBadgeProps
>(({ labels, className, children, ...props }, ref) => {
  const { schema } = useProcedureContext();

  const defaultLabels: Record<string, string> = {
    query: "Query",
    mutation: "Mutation",
    subscription: "Subscription",
  };

  const label =
    children ??
    (labels ?? defaultLabels)[schema.procedure_type] ??
    schema.procedure_type;

  return (
    <span
      ref={ref}
      className={className}
      data-type={schema.procedure_type}
      data-testid="api-docs-procedure-badge"
      {...props}
    >
      {label}
    </span>
  );
});
ApiDocsProcedureBadge.displayName = "ApiDocsProcedureBadge";

/** Procedure path display */
export const ApiDocsProcedurePath = forwardRef<
  HTMLSpanElement,
  HTMLAttributes<HTMLSpanElement>
>(({ className, children, ...props }, ref) => {
  const { path } = useProcedureContext();

  return (
    <span
      ref={ref}
      className={className}
      data-testid="api-docs-procedure-path"
      {...props}
    >
      {children ?? path}
    </span>
  );
});
ApiDocsProcedurePath.displayName = "ApiDocsProcedurePath";

/** Deprecated indicator */
export const ApiDocsProcedureDeprecated = forwardRef<
  HTMLSpanElement,
  HTMLAttributes<HTMLSpanElement>
>(({ className, children, ...props }, ref) => {
  const { schema } = useProcedureContext();

  if (!schema.deprecated) return null;

  return (
    <span
      ref={ref}
      className={className}
      data-testid="api-docs-procedure-deprecated"
      {...props}
    >
      {children ?? "Deprecated"}
    </span>
  );
});
ApiDocsProcedureDeprecated.displayName = "ApiDocsProcedureDeprecated";

/** Procedure description */
export const ApiDocsProcedureDescription = forwardRef<
  HTMLParagraphElement,
  HTMLAttributes<HTMLParagraphElement>
>(({ className, children, ...props }, ref) => {
  const { schema } = useProcedureContext();

  if (!schema.description && !children) return null;

  return (
    <p
      ref={ref}
      className={className}
      data-testid="api-docs-procedure-description"
      {...props}
    >
      {children ?? schema.description}
    </p>
  );
});
ApiDocsProcedureDescription.displayName = "ApiDocsProcedureDescription";

/** Expandable details container - only renders when expanded */
export const ApiDocsProcedureDetails = forwardRef<
  HTMLDivElement,
  HTMLAttributes<HTMLDivElement>
>(({ className, children, ...props }, ref) => {
  const { expanded } = useProcedureContext();

  if (!expanded) return null;

  return (
    <div
      ref={ref}
      className={className}
      data-testid="api-docs-procedure-details"
      {...props}
    >
      {children}
    </div>
  );
});
ApiDocsProcedureDetails.displayName = "ApiDocsProcedureDetails";

export interface ApiDocsProcedureTagsProps extends HTMLAttributes<HTMLDivElement> {
  /** Render prop for custom tag rendering */
  renderTag?: (tag: string) => ReactNode;
}

/** Procedure tags display */
export const ApiDocsProcedureTags = forwardRef<
  HTMLDivElement,
  ApiDocsProcedureTagsProps
>(({ renderTag, className, children, ...props }, ref) => {
  const { schema } = useProcedureContext();

  if (schema.tags.length === 0) return null;

  return (
    <div
      ref={ref}
      className={className}
      data-testid="api-docs-procedure-tags"
      {...props}
    >
      {children ??
        schema.tags.map((tag) =>
          renderTag ? renderTag(tag) : <span key={tag}>{tag}</span>,
        )}
    </div>
  );
});
ApiDocsProcedureTags.displayName = "ApiDocsProcedureTags";

export interface ApiDocsProcedureSchemaProps extends HTMLAttributes<HTMLDivElement> {
  /** Which schema to display */
  type: "input" | "output";
  /** Label for the schema section */
  label?: string;
}

/** Schema section (input or output) */
export const ApiDocsProcedureSchema = forwardRef<
  HTMLDivElement,
  ApiDocsProcedureSchemaProps
>(({ type, label, className, children, ...props }, ref) => {
  const { schema } = useProcedureContext();
  const typeSchema = type === "input" ? schema.input : schema.output;

  if (!typeSchema) return null;

  return (
    <div
      ref={ref}
      className={className}
      data-testid={`api-docs-procedure-${type}`}
      {...props}
    >
      {label && <span>{label}</span>}
      {children}
    </div>
  );
});
ApiDocsProcedureSchema.displayName = "ApiDocsProcedureSchema";

/** Hook to access current procedure context */
export function useCurrentProcedure() {
  return useProcedureContext();
}
