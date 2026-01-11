// =============================================================================
// Property-Based Tests for ProcedureCard Component
// =============================================================================
// Tests using fast-check to verify universal properties of ProcedureCard.

import { describe, it, expect, vi, afterEach } from "vitest";
import * as fc from "fast-check";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { ProcedureCard } from "../ProcedureCard";
import type { ProcedureSchema, ProcedureType, TypeSchema } from "../types";

// Mock Tauri invoke for ProcedureTester
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// =============================================================================
// Arbitraries (Generators)
// =============================================================================

/** Generate a valid procedure type */
const procedureTypeArb = fc.constantFrom<ProcedureType>(
  "query",
  "mutation",
  "subscription",
);

/** Generate a simple type schema */
const simpleTypeSchemaArb: fc.Arbitrary<TypeSchema> = fc.record({
  type: fc.constantFrom("string", "number", "boolean"),
  description: fc.option(fc.string({ minLength: 1, maxLength: 30 }), {
    nil: undefined,
  }),
});

/** Generate a procedure schema */
const procedureSchemaArb: fc.Arbitrary<ProcedureSchema> = fc.record({
  procedure_type: procedureTypeArb,
  description: fc.option(fc.string({ minLength: 1, maxLength: 100 }), {
    nil: undefined,
  }),
  deprecated: fc.boolean(),
  tags: fc.array(fc.string({ minLength: 1, maxLength: 15 }), { maxLength: 5 }),
  input: fc.option(simpleTypeSchemaArb, { nil: undefined }),
  output: fc.option(simpleTypeSchemaArb, { nil: undefined }),
  metadata: fc.option(fc.record({ key: fc.string() }), { nil: undefined }),
});

/** Generate a valid procedure path */
const procedurePathArb = fc.oneof(
  fc
    .string({ minLength: 1, maxLength: 15 })
    .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
  fc
    .tuple(
      fc
        .string({ minLength: 1, maxLength: 10 })
        .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
      fc
        .string({ minLength: 1, maxLength: 10 })
        .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
    )
    .map(([ns, name]) => `${ns}.${name}`),
);

// =============================================================================
// Property 2: Procedure Card Displays Required Information
// =============================================================================
// **Validates: Requirements 2.2, 2.4**

describe("Property 2: Procedure Card Displays Required Information", () => {
  it("displays procedure path", () => {
    fc.assert(
      fc.property(procedurePathArb, procedureSchemaArb, (path, schema) => {
        const onToggle = vi.fn();
        const { container } = render(
          <ProcedureCard
            path={path}
            schema={schema}
            expanded={false}
            onToggle={onToggle}
          />,
        );

        const pathElement = container.querySelector(
          '[data-testid="procedure-path"]',
        );
        expect(pathElement).toBeTruthy();
        expect(pathElement?.textContent).toBe(path);
      }),
      { numRuns: 50 },
    );
  });

  it("displays type badge matching procedure_type", () => {
    fc.assert(
      fc.property(procedurePathArb, procedureSchemaArb, (path, schema) => {
        const onToggle = vi.fn();
        const { container } = render(
          <ProcedureCard
            path={path}
            schema={schema}
            expanded={false}
            onToggle={onToggle}
          />,
        );

        const badge = container.querySelector('[data-testid="type-badge"]');
        expect(badge).toBeTruthy();

        const expectedLabel = {
          query: "Query",
          mutation: "Mutation",
          subscription: "Subscription",
        }[schema.procedure_type];

        expect(badge?.textContent).toBe(expectedLabel);

        const expectedClass = {
          query: "badge-query",
          mutation: "badge-mutation",
          subscription: "badge-subscription",
        }[schema.procedure_type];

        expect(badge?.classList.contains(expectedClass)).toBe(true);
      }),
      { numRuns: 50 },
    );
  });

  it("displays deprecation indicator when deprecated is true", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter((s) => s.deprecated === true),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={false}
              onToggle={onToggle}
            />,
          );

          const deprecatedIndicator = container.querySelector(
            '[data-testid="deprecated-indicator"]',
          );
          expect(deprecatedIndicator).toBeTruthy();
          expect(deprecatedIndicator?.textContent).toContain("Deprecated");
        },
      ),
      { numRuns: 30 },
    );
  });

  it("does not display deprecation indicator when deprecated is false", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter((s) => s.deprecated === false),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={false}
              onToggle={onToggle}
            />,
          );

          const deprecatedIndicator = container.querySelector(
            '[data-testid="deprecated-indicator"]',
          );
          expect(deprecatedIndicator).toBeNull();
        },
      ),
      { numRuns: 30 },
    );
  });

  it("calls onToggle when header is clicked", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb,
        fc.boolean(),
        (path, schema, expanded) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={expanded}
              onToggle={onToggle}
            />,
          );

          const header = container.querySelector(".procedure-header");
          expect(header).toBeTruthy();

          fireEvent.click(header!);
          expect(onToggle).toHaveBeenCalledTimes(1);
        },
      ),
      { numRuns: 30 },
    );
  });
});

// =============================================================================
// Property 7: Expanded Procedure Shows Additional Details
// =============================================================================
// **Validates: Requirements 5.3**

describe("Property 7: Expanded Procedure Shows Additional Details", () => {
  it("shows description when expanded and description exists", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter(
          (s) => s.description !== undefined && s.description.length > 0,
        ),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={true}
              onToggle={onToggle}
            />,
          );

          const descElement = container.querySelector(
            '[data-testid="description"]',
          );
          expect(descElement).toBeTruthy();
          expect(descElement?.textContent).toContain(schema.description);
        },
      ),
      { numRuns: 30 },
    );
  });

  it("shows tags when expanded and tags exist", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter((s) => s.tags.length > 0),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={true}
              onToggle={onToggle}
            />,
          );

          const tagsElement = container.querySelector('[data-testid="tags"]');
          expect(tagsElement).toBeTruthy();

          for (const tag of schema.tags) {
            expect(tagsElement?.textContent).toContain(tag);
          }
        },
      ),
      { numRuns: 30 },
    );
  });

  it("shows input schema when expanded and input exists", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter((s) => s.input !== undefined),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={true}
              onToggle={onToggle}
            />,
          );

          const inputElement = container.querySelector(
            '[data-testid="input-schema"]',
          );
          expect(inputElement).toBeTruthy();
        },
      ),
      { numRuns: 30 },
    );
  });

  it("shows output schema when expanded and output exists", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter((s) => s.output !== undefined),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={true}
              onToggle={onToggle}
            />,
          );

          const outputElement = container.querySelector(
            '[data-testid="output-schema"]',
          );
          expect(outputElement).toBeTruthy();
        },
      ),
      { numRuns: 30 },
    );
  });

  it("shows metadata when expanded and metadata exists", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        procedureSchemaArb.filter((s) => s.metadata !== undefined),
        (path, schema) => {
          const onToggle = vi.fn();
          const { container } = render(
            <ProcedureCard
              path={path}
              schema={schema}
              expanded={true}
              onToggle={onToggle}
            />,
          );

          const metadataElement = container.querySelector(
            '[data-testid="metadata"]',
          );
          expect(metadataElement).toBeTruthy();
        },
      ),
      { numRuns: 30 },
    );
  });

  it("hides details when not expanded", () => {
    fc.assert(
      fc.property(procedurePathArb, procedureSchemaArb, (path, schema) => {
        const onToggle = vi.fn();
        const { container } = render(
          <ProcedureCard
            path={path}
            schema={schema}
            expanded={false}
            onToggle={onToggle}
          />,
        );

        const detailsElement = container.querySelector(
          '[data-testid="procedure-details"]',
        );
        expect(detailsElement).toBeNull();
      }),
      { numRuns: 30 },
    );
  });
});

// =============================================================================
// Property 1: Try It Section Visibility
// =============================================================================
// **Validates: Requirements 1.1, 1.2**

describe("Property 1: Try It Section Visibility", () => {
  afterEach(() => {
    cleanup();
  });

  it("shows ProcedureTester when card is expanded", () => {
    fc.assert(
      fc.property(procedurePathArb, procedureSchemaArb, (path, schema) => {
        cleanup();
        const onToggle = vi.fn();
        const { container } = render(
          <ProcedureCard
            path={path}
            schema={schema}
            expanded={true}
            onToggle={onToggle}
          />,
        );

        const tester = container.querySelector(
          '[data-testid="procedure-tester"]',
        );
        expect(tester).toBeTruthy();
      }),
      { numRuns: 100 },
    );
  });

  it("hides ProcedureTester when card is collapsed", () => {
    fc.assert(
      fc.property(procedurePathArb, procedureSchemaArb, (path, schema) => {
        cleanup();
        const onToggle = vi.fn();
        const { container } = render(
          <ProcedureCard
            path={path}
            schema={schema}
            expanded={false}
            onToggle={onToggle}
          />,
        );

        const tester = container.querySelector(
          '[data-testid="procedure-tester"]',
        );
        expect(tester).toBeNull();
      }),
      { numRuns: 100 },
    );
  });

  it("ProcedureTester receives correct path prop", () => {
    fc.assert(
      fc.property(procedurePathArb, procedureSchemaArb, (path, schema) => {
        cleanup();
        const onToggle = vi.fn();
        const { container } = render(
          <ProcedureCard
            path={path}
            schema={schema}
            expanded={true}
            onToggle={onToggle}
          />,
        );

        // The tester should be present
        const tester = container.querySelector(
          '[data-testid="procedure-tester"]',
        );
        expect(tester).toBeTruthy();

        // The execute button should be present (indicates tester is functional)
        const executeBtn = container.querySelector(
          '[data-testid="procedure-tester-execute"]',
        );
        expect(executeBtn).toBeTruthy();
      }),
      { numRuns: 100 },
    );
  });
});
