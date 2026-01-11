// =============================================================================
// TypeRenderer Component
// =============================================================================
// Renders TypeSchema objects as readable type definitions with syntax highlighting.

import { JSX } from 'react';
import type { TypeSchema } from './types';

export interface TypeRendererProps {
  /** The type schema to render */
  schema: TypeSchema;
  /** Nesting level for indentation (default: 0) */
  level?: number;
  /** Whether to show examples (default: true) */
  showExamples?: boolean;
  /** Maximum nesting depth before truncation (default: 10) */
  maxDepth?: number;
}

/**
 * Render constraints for a type schema.
 */
function renderConstraints(schema: TypeSchema): string[] {
  const constraints: string[] = [];

  if (schema.minimum !== undefined) {
    constraints.push(`min: ${schema.minimum}`);
  }
  if (schema.maximum !== undefined) {
    constraints.push(`max: ${schema.maximum}`);
  }
  if (schema.minLength !== undefined) {
    constraints.push(`minLength: ${schema.minLength}`);
  }
  if (schema.maxLength !== undefined) {
    constraints.push(`maxLength: ${schema.maxLength}`);
  }
  if (schema.pattern) {
    constraints.push(`pattern: ${schema.pattern}`);
  }
  if (schema.format) {
    constraints.push(`format: ${schema.format}`);
  }
  if (schema.nullable) {
    constraints.push('nullable');
  }

  return constraints;
}

/**
 * Get the display type name with any enum values.
 */
function getTypeName(schema: TypeSchema): string {
  if (schema.enum && schema.enum.length > 0) {
    const enumValues = schema.enum.map(v => JSON.stringify(v)).join(' | ');
    return enumValues;
  }
  return schema.type;
}

/**
 * TypeRenderer component for displaying type schemas.
 */
export function TypeRenderer({
  schema,
  level = 0,
  showExamples = true,
  maxDepth = 10,
}: TypeRendererProps): JSX.Element {
  const indent = '  '.repeat(level);

  // Truncate deeply nested types
  if (level >= maxDepth) {
    return <span className="type-truncated">...</span>;
  }

  const constraints = renderConstraints(schema);
  const typeName = getTypeName(schema);

  // Handle object types
  if (schema.type === 'object' && schema.properties) {
    const properties = Object.entries(schema.properties);
    const required = new Set(schema.required ?? []);

    return (
      <div className="type-object" data-testid="type-object">
        <span className="type-brace">{'{'}</span>
        {properties.length > 0 && (
          <div className="type-properties">
            {properties.map(([name, propSchema]) => (
              <div key={name} className="type-property" data-testid={`property-${name}`}>
                <span className="type-indent">{indent}  </span>
                <span className="type-property-name">{name}</span>
                {required.has(name) && (
                  <span className="type-required" title="Required">*</span>
                )}
                <span className="type-colon">: </span>
                <TypeRenderer
                  schema={propSchema}
                  level={level + 1}
                  showExamples={showExamples}
                  maxDepth={maxDepth}
                />
                {propSchema.description && (
                  <span className="type-description"> // {propSchema.description}</span>
                )}
              </div>
            ))}
          </div>
        )}
        <span className="type-indent">{indent}</span>
        <span className="type-brace">{'}'}</span>
        {constraints.length > 0 && (
          <span className="type-constraints" data-testid="constraints">
            {' '}({constraints.join(', ')})
          </span>
        )}
        {showExamples && schema.example !== undefined && (
          <span className="type-example" data-testid="example">
            {' '}// e.g. {JSON.stringify(schema.example)}
          </span>
        )}
      </div>
    );
  }

  // Handle array types
  if (schema.type === 'array' && schema.items) {
    return (
      <span className="type-array" data-testid="type-array">
        <TypeRenderer
          schema={schema.items}
          level={level}
          showExamples={false}
          maxDepth={maxDepth}
        />
        <span className="type-array-brackets">[]</span>
        {constraints.length > 0 && (
          <span className="type-constraints" data-testid="constraints">
            {' '}({constraints.join(', ')})
          </span>
        )}
        {showExamples && schema.example !== undefined && (
          <span className="type-example" data-testid="example">
            {' '}// e.g. {JSON.stringify(schema.example)}
          </span>
        )}
      </span>
    );
  }

  // Handle primitive types
  return (
    <span className={`type-primitive type-${schema.type}`} data-testid={`type-${schema.type}`}>
      <span className="type-name">{typeName}</span>
      {constraints.length > 0 && (
        <span className="type-constraints" data-testid="constraints">
          {' '}({constraints.join(', ')})
        </span>
      )}
      {showExamples && schema.example !== undefined && (
        <span className="type-example" data-testid="example">
          {' '}// e.g. {JSON.stringify(schema.example)}
        </span>
      )}
    </span>
  );
}

export default TypeRenderer;
