// =============================================================================
// ApiDocsTypeRenderer - Type Schema Rendering
// =============================================================================

import { type HTMLAttributes, forwardRef, type ReactNode } from "react";
import type { TypeSchema } from "../types";

export interface ApiDocsTypeRendererProps extends HTMLAttributes<HTMLDivElement> {
  /** Type schema to render */
  schema: TypeSchema;
  /** Nesting level (default: 0) */
  level?: number;
  /** Show examples (default: true) */
  showExamples?: boolean;
  /** Max depth before truncation (default: 10) */
  maxDepth?: number;
  /** Custom renderers for different parts */
  slots?: {
    /** Render object braces */
    brace?: (props: { char: "{" | "}" }) => ReactNode;
    /** Render property name */
    propertyName?: (props: { name: string; required: boolean }) => ReactNode;
    /** Render type name */
    typeName?: (props: { type: string; schema: TypeSchema }) => ReactNode;
    /** Render constraints */
    constraints?: (props: { constraints: string[] }) => ReactNode;
    /** Render example */
    example?: (props: { value: unknown }) => ReactNode;
    /** Render description */
    description?: (props: { text: string }) => ReactNode;
    /** Render array brackets */
    arrayBrackets?: () => ReactNode;
    /** Render truncation indicator */
    truncated?: () => ReactNode;
  };
}

function getConstraints(schema: TypeSchema): string[] {
  const constraints: string[] = [];
  if (schema.minimum !== undefined) constraints.push(`min: ${schema.minimum}`);
  if (schema.maximum !== undefined) constraints.push(`max: ${schema.maximum}`);
  if (schema.minLength !== undefined)
    constraints.push(`minLength: ${schema.minLength}`);
  if (schema.maxLength !== undefined)
    constraints.push(`maxLength: ${schema.maxLength}`);
  if (schema.pattern) constraints.push(`pattern: ${schema.pattern}`);
  if (schema.format) constraints.push(`format: ${schema.format}`);
  if (schema.nullable) constraints.push("nullable");
  return constraints;
}

function getTypeName(schema: TypeSchema): string {
  if (schema.enum && schema.enum.length > 0) {
    return schema.enum.map((v) => JSON.stringify(v)).join(" | ");
  }
  return schema.type;
}

/** Headless type schema renderer */
export const ApiDocsTypeRenderer = forwardRef<
  HTMLDivElement,
  ApiDocsTypeRendererProps
>(
  (
    {
      schema,
      level = 0,
      showExamples = true,
      maxDepth = 10,
      slots,
      className,
      ...props
    },
    ref,
  ) => {
    const indent = "  ".repeat(level);
    const constraints = getConstraints(schema);
    const typeName = getTypeName(schema);

    // Truncate deeply nested types
    if (level >= maxDepth) {
      return (
        <span className={className} data-testid="type-truncated" {...props}>
          {slots?.truncated?.() ?? "..."}
        </span>
      );
    }

    // Object type
    if (schema.type === "object" && schema.properties) {
      const properties = Object.entries(schema.properties);
      const required = new Set(schema.required ?? []);

      return (
        <div
          ref={ref}
          className={className}
          data-testid="type-object"
          {...props}
        >
          {slots?.brace?.({ char: "{" }) ?? <span>{"{"}</span>}
          {properties.length > 0 && (
            <div>
              {properties.map(([name, propSchema]) => (
                <div key={name} data-testid={`type-property-${name}`}>
                  <span>{indent} </span>
                  {slots?.propertyName?.({
                    name,
                    required: required.has(name),
                  }) ?? (
                    <>
                      <span data-property-name>{name}</span>
                      {required.has(name) && <span data-required>*</span>}
                    </>
                  )}
                  <span>: </span>
                  <ApiDocsTypeRenderer
                    schema={propSchema}
                    level={level + 1}
                    showExamples={showExamples}
                    maxDepth={maxDepth}
                    slots={slots}
                  />
                  {propSchema.description &&
                    (slots?.description?.({ text: propSchema.description }) ?? (
                      <span data-description> // {propSchema.description}</span>
                    ))}
                </div>
              ))}
            </div>
          )}
          <span>{indent}</span>
          {slots?.brace?.({ char: "}" }) ?? <span>{"}"}</span>}
          {constraints.length > 0 &&
            (slots?.constraints?.({ constraints }) ?? (
              <span data-constraints> ({constraints.join(", ")})</span>
            ))}
          {showExamples &&
            schema.example !== undefined &&
            (slots?.example?.({ value: schema.example }) ?? (
              <span data-example>
                {" "}
                // e.g. {JSON.stringify(schema.example)}
              </span>
            ))}
        </div>
      );
    }

    // Array type
    if (schema.type === "array" && schema.items) {
      return (
        <span
          ref={ref}
          className={className}
          data-testid="type-array"
          {...props}
        >
          <ApiDocsTypeRenderer
            schema={schema.items}
            level={level}
            showExamples={false}
            maxDepth={maxDepth}
            slots={slots}
          />
          {slots?.arrayBrackets?.() ?? <span>[]</span>}
          {constraints.length > 0 &&
            (slots?.constraints?.({ constraints }) ?? (
              <span data-constraints> ({constraints.join(", ")})</span>
            ))}
          {showExamples &&
            schema.example !== undefined &&
            (slots?.example?.({ value: schema.example }) ?? (
              <span data-example>
                {" "}
                // e.g. {JSON.stringify(schema.example)}
              </span>
            ))}
        </span>
      );
    }

    // Primitive type
    return (
      <span
        ref={ref}
        className={className}
        data-type={schema.type}
        data-testid={`type-${schema.type}`}
        {...props}
      >
        {slots?.typeName?.({ type: typeName, schema }) ?? (
          <span>{typeName}</span>
        )}
        {constraints.length > 0 &&
          (slots?.constraints?.({ constraints }) ?? (
            <span data-constraints> ({constraints.join(", ")})</span>
          ))}
        {showExamples &&
          schema.example !== undefined &&
          (slots?.example?.({ value: schema.example }) ?? (
            <span data-example> // e.g. {JSON.stringify(schema.example)}</span>
          ))}
      </span>
    );
  },
);
ApiDocsTypeRenderer.displayName = "ApiDocsTypeRenderer";
