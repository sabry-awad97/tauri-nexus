// =============================================================================
// Property-Based Tests for TypeRenderer Component
// =============================================================================
// Tests using fast-check to verify universal properties of TypeRenderer.

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import { render, screen } from "@testing-library/react";
import { TypeRenderer } from "../TypeRenderer";
import type { TypeSchema } from "../types";

// =============================================================================
// Arbitraries (Generators)
// =============================================================================

/** Generate a primitive type name */
const primitiveTypeArb = fc.constantFrom(
  "string",
  "number",
  "integer",
  "boolean",
  "null",
);

/** Generate a simple primitive type schema */
const primitiveSchemaArb: fc.Arbitrary<TypeSchema> = fc.record({
  type: primitiveTypeArb,
  description: fc.option(fc.string({ minLength: 1, maxLength: 50 }), {
    nil: undefined,
  }),
  example: fc.option(fc.oneof(fc.string(), fc.integer(), fc.boolean()), {
    nil: undefined,
  }),
  minimum: fc.option(fc.integer({ min: -1000, max: 1000 }), { nil: undefined }),
  maximum: fc.option(fc.integer({ min: -1000, max: 1000 }), { nil: undefined }),
  minLength: fc.option(fc.integer({ min: 0, max: 100 }), { nil: undefined }),
  maxLength: fc.option(fc.integer({ min: 0, max: 1000 }), { nil: undefined }),
  pattern: fc.option(fc.constant("^[a-z]+$"), { nil: undefined }),
  format: fc.option(fc.constantFrom("email", "uuid", "date-time"), {
    nil: undefined,
  }),
  nullable: fc.option(fc.boolean(), { nil: undefined }),
  enum: fc.option(
    fc.array(fc.string({ minLength: 1, maxLength: 10 }), {
      minLength: 1,
      maxLength: 5,
    }),
    { nil: undefined },
  ),
});

/** Generate a property name */
const propertyNameArb = fc
  .string({ minLength: 1, maxLength: 20 })
  .filter((s) => /^[a-zA-Z][a-zA-Z0-9_]*$/.test(s));

/** Generate an object type schema with properties */
const objectSchemaArb: fc.Arbitrary<TypeSchema> = fc
  .array(fc.tuple(propertyNameArb, primitiveSchemaArb), {
    minLength: 1,
    maxLength: 5,
  })
  .chain((props) => {
    const properties: Record<string, TypeSchema> = {};
    const propNames: string[] = [];
    for (const [name, schema] of props) {
      if (!properties[name]) {
        properties[name] = schema;
        propNames.push(name);
      }
    }
    return fc.record({
      type: fc.constant("object" as const),
      properties: fc.constant(properties),
      required: fc.subarray(propNames),
      description: fc.option(fc.string({ minLength: 1, maxLength: 50 }), {
        nil: undefined,
      }),
      example: fc.option(fc.constant({ sample: "value" }), { nil: undefined }),
    });
  });

/** Generate an array type schema */
const arraySchemaArb: fc.Arbitrary<TypeSchema> = primitiveSchemaArb.map(
  (itemSchema) => ({
    type: "array",
    items: itemSchema,
    minLength: undefined,
    maxLength: undefined,
  }),
);

/** Generate any type schema */
const anySchemaArb = fc.oneof(
  primitiveSchemaArb,
  objectSchemaArb,
  arraySchemaArb,
);

// =============================================================================
// Property 3: TypeRenderer Displays All Schema Properties
// =============================================================================
// **Validates: Requirements 3.1, 3.2**

describe("Property 3: TypeRenderer Displays All Schema Properties", () => {
  it("renders all property names for object types", () => {
    fc.assert(
      fc.property(objectSchemaArb, (schema) => {
        const { container } = render(<TypeRenderer schema={schema} />);

        if (schema.properties) {
          for (const propName of Object.keys(schema.properties)) {
            const propElement = container.querySelector(
              `[data-testid="property-${propName}"]`,
            );
            expect(propElement).toBeTruthy();
            expect(propElement?.textContent).toContain(propName);
          }
        }
      }),
      { numRuns: 50 },
    );
  });

  it("marks required properties with indicator", () => {
    fc.assert(
      fc.property(objectSchemaArb, (schema) => {
        const { container } = render(<TypeRenderer schema={schema} />);

        const required = new Set(schema.required ?? []);
        if (schema.properties) {
          for (const propName of Object.keys(schema.properties)) {
            const propElement = container.querySelector(
              `[data-testid="property-${propName}"]`,
            );
            if (propElement) {
              const hasRequiredIndicator =
                propElement.querySelector(".type-required") !== null;
              expect(hasRequiredIndicator).toBe(required.has(propName));
            }
          }
        }
      }),
      { numRuns: 50 },
    );
  });

  it("renders type name for primitive types", () => {
    fc.assert(
      fc.property(primitiveSchemaArb, (schema) => {
        const { container } = render(<TypeRenderer schema={schema} />);

        // Should contain the type name or enum values
        if (schema.enum && schema.enum.length > 0) {
          // Enum values should be displayed
          for (const val of schema.enum) {
            expect(container.textContent).toContain(JSON.stringify(val));
          }
        } else {
          expect(container.textContent).toContain(schema.type);
        }
      }),
      { numRuns: 50 },
    );
  });

  it("renders array types with brackets", () => {
    fc.assert(
      fc.property(arraySchemaArb, (schema) => {
        const { container } = render(<TypeRenderer schema={schema} />);

        expect(container.textContent).toContain("[]");
      }),
      { numRuns: 50 },
    );
  });
});

// =============================================================================
// Property 4: TypeRenderer Displays Constraints and Examples
// =============================================================================
// **Validates: Requirements 3.4, 3.5**

describe("Property 4: TypeRenderer Displays Constraints and Examples", () => {
  it("displays minimum constraint when present", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.minimum !== undefined),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain(`min: ${schema.minimum}`);
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays maximum constraint when present", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.maximum !== undefined),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain(`max: ${schema.maximum}`);
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays minLength constraint when present", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.minLength !== undefined),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain(
            `minLength: ${schema.minLength}`,
          );
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays maxLength constraint when present", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.maxLength !== undefined),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain(
            `maxLength: ${schema.maxLength}`,
          );
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays format constraint when present", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.format !== undefined),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain(`format: ${schema.format}`);
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays pattern constraint when present", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.pattern !== undefined),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain(`pattern: ${schema.pattern}`);
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays nullable indicator when true", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.nullable === true),
        (schema) => {
          const { container } = render(<TypeRenderer schema={schema} />);
          expect(container.textContent).toContain("nullable");
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays example when present and showExamples is true", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.example !== undefined),
        (schema) => {
          const { container } = render(
            <TypeRenderer schema={schema} showExamples={true} />,
          );
          expect(container.textContent).toContain("e.g.");
          expect(container.textContent).toContain(
            JSON.stringify(schema.example),
          );
        },
      ),
      { numRuns: 30 },
    );
  });

  it("hides example when showExamples is false", () => {
    fc.assert(
      fc.property(
        primitiveSchemaArb.filter((s) => s.example !== undefined),
        (schema) => {
          const { container } = render(
            <TypeRenderer schema={schema} showExamples={false} />,
          );
          expect(container.textContent).not.toContain("e.g.");
        },
      ),
      { numRuns: 30 },
    );
  });
});
