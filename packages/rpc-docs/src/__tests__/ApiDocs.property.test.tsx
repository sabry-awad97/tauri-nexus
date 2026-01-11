// =============================================================================
// Property-Based Tests for ApiDocs Component
// =============================================================================
// Tests using fast-check to verify universal properties of ApiDocs.

import { describe, it, expect, vi, beforeEach } from "vitest";
import * as fc from "fast-check";
import { render, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiDocs } from "../ApiDocs";
import type { RouterSchema, ProcedureSchema, ProcedureType } from "../types";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

// =============================================================================
// Test Utilities
// =============================================================================

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });
}

function renderWithClient(ui: React.ReactElement, client: QueryClient) {
  return render(
    <QueryClientProvider client={client}>{ui}</QueryClientProvider>,
  );
}

// =============================================================================
// Arbitraries (Generators)
// =============================================================================

/** Generate a valid procedure type */
const procedureTypeArb = fc.constantFrom<ProcedureType>(
  "query",
  "mutation",
  "subscription",
);

/** Generate a procedure schema */
const procedureSchemaArb: fc.Arbitrary<ProcedureSchema> = fc.record({
  procedure_type: procedureTypeArb,
  description: fc.option(fc.string({ minLength: 1, maxLength: 50 }), {
    nil: undefined,
  }),
  deprecated: fc.boolean(),
  tags: fc.array(fc.string({ minLength: 1, maxLength: 10 }), { maxLength: 3 }),
  input: fc.constant(undefined),
  output: fc.constant(undefined),
  metadata: fc.constant(undefined),
});

/** Generate procedure names (what the backend actually returns) */
const procedureNamesArb: fc.Arbitrary<string[]> = fc
  .array(
    fc
      .string({ minLength: 1, maxLength: 15 })
      .filter((s) => /^[a-z][a-z0-9.]*$/i.test(s)),
    { minLength: 1, maxLength: 10 },
  )
  .map((names) => [...new Set(names)]); // Remove duplicates

/** Generate a router schema (for reference in tests) */
const routerSchemaArb: fc.Arbitrary<RouterSchema> = fc
  .array(
    fc.tuple(
      fc
        .string({ minLength: 1, maxLength: 15 })
        .filter((s) => /^[a-z][a-z0-9.]*$/i.test(s)),
      procedureSchemaArb,
    ),
    { minLength: 1, maxLength: 10 },
  )
  .map((entries) => {
    const procedures: Record<string, ProcedureSchema> = {};
    for (const [path, schema] of entries) {
      if (!procedures[path]) {
        procedures[path] = schema;
      }
    }
    return {
      version: "1.0.0",
      name: "Test API",
      description: "Test API description",
      procedures,
    };
  });

/** Generate custom title and description */
const customPropsArb = fc.record({
  title: fc.string({ minLength: 1, maxLength: 50 }),
  description: fc.string({ minLength: 1, maxLength: 100 }),
});

// =============================================================================
// Property 8: Props Customize Component Output
// =============================================================================
// **Validates: Requirements 6.2**

describe("Property 8: Props Customize Component Output", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("displays custom title when provided", async () => {
    await fc.assert(
      fc.asyncProperty(
        procedureNamesArb,
        customPropsArb,
        async (procedureNames, props) => {
          vi.mocked(invoke).mockResolvedValue(procedureNames);
          const client = createTestQueryClient();

          const { container } = renderWithClient(
            <ApiDocs title={props.title} />,
            client,
          );

          await waitFor(() => {
            const titleElement = container.querySelector(
              '[data-testid="api-docs-title"]',
            );
            expect(titleElement).toBeTruthy();
            expect(titleElement?.textContent).toBe(props.title);
          });
        },
      ),
      { numRuns: 20 },
    );
  });

  it("displays custom description when provided", async () => {
    await fc.assert(
      fc.asyncProperty(
        procedureNamesArb,
        customPropsArb,
        async (procedureNames, props) => {
          vi.mocked(invoke).mockResolvedValue(procedureNames);
          const client = createTestQueryClient();

          const { container } = renderWithClient(
            <ApiDocs description={props.description} />,
            client,
          );

          await waitFor(() => {
            const descElement = container.querySelector(
              '[data-testid="api-docs-description"]',
            );
            expect(descElement).toBeTruthy();
            expect(descElement?.textContent).toBe(props.description);
          });
        },
      ),
      { numRuns: 20 },
    );
  });

  it("hides header when showHeader is false", async () => {
    await fc.assert(
      fc.asyncProperty(procedureNamesArb, async (procedureNames) => {
        vi.mocked(invoke).mockResolvedValue(procedureNames);
        const client = createTestQueryClient();

        const { container } = renderWithClient(
          <ApiDocs showHeader={false} />,
          client,
        );

        await waitFor(() => {
          const docsElement = container.querySelector(
            '[data-testid="api-docs"]',
          );
          expect(docsElement).toBeTruthy();
        });

        const headerElement = container.querySelector(
          '[data-testid="api-docs-header"]',
        );
        expect(headerElement).toBeNull();
      }),
      { numRuns: 20 },
    );
  });

  it("shows header when showHeader is true (default)", async () => {
    await fc.assert(
      fc.asyncProperty(procedureNamesArb, async (procedureNames) => {
        vi.mocked(invoke).mockResolvedValue(procedureNames);
        const client = createTestQueryClient();

        const { container } = renderWithClient(
          <ApiDocs showHeader={true} />,
          client,
        );

        await waitFor(() => {
          const headerElement = container.querySelector(
            '[data-testid="api-docs-header"]',
          );
          expect(headerElement).toBeTruthy();
        });
      }),
      { numRuns: 20 },
    );
  });

  it("applies custom className", async () => {
    await fc.assert(
      fc.asyncProperty(
        procedureNamesArb,
        fc
          .string({ minLength: 1, maxLength: 20 })
          .filter((s) => /^[a-z][a-z0-9-]*$/i.test(s)),
        async (procedureNames, customClass) => {
          vi.mocked(invoke).mockResolvedValue(procedureNames);
          const client = createTestQueryClient();

          const { container } = renderWithClient(
            <ApiDocs className={customClass} />,
            client,
          );

          await waitFor(() => {
            const docsElement = container.querySelector(
              '[data-testid="api-docs"]',
            );
            expect(docsElement).toBeTruthy();
            expect(docsElement?.classList.contains(customClass)).toBe(true);
          });
        },
      ),
      { numRuns: 20 },
    );
  });

  it("uses default title when not provided", async () => {
    await fc.assert(
      fc.asyncProperty(procedureNamesArb, async (procedureNames) => {
        vi.mocked(invoke).mockResolvedValue(procedureNames);
        const client = createTestQueryClient();

        const { container } = renderWithClient(<ApiDocs />, client);

        await waitFor(() => {
          const titleElement = container.querySelector(
            '[data-testid="api-docs-title"]',
          );
          expect(titleElement).toBeTruthy();
          expect(titleElement?.textContent).toBe("API Documentation");
        });
      }),
      { numRuns: 10 },
    );
  });
});
