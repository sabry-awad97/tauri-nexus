// =============================================================================
// Property-Based Tests for Documentation Utilities
// =============================================================================
// Tests using fast-check to verify universal properties of utility functions.

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import { groupProcedures, filterProcedures } from "../utils";
import type { ProcedureSchema, ProcedureType } from "../types";

// =============================================================================
// Arbitraries (Generators)
// =============================================================================

/** Generate a valid procedure type */
const procedureTypeArb = fc.constantFrom<ProcedureType>(
  "query",
  "mutation",
  "subscription",
);

/** Generate a valid procedure schema */
const procedureSchemaArb: fc.Arbitrary<ProcedureSchema> = fc.record({
  procedure_type: procedureTypeArb,
  description: fc.option(fc.string({ minLength: 0, maxLength: 100 }), {
    nil: undefined,
  }),
  deprecated: fc.boolean(),
  tags: fc.array(fc.string({ minLength: 1, maxLength: 20 }), { maxLength: 5 }),
  input: fc.constant(undefined),
  output: fc.constant(undefined),
  metadata: fc.constant(undefined),
});

/** Generate a valid procedure path (e.g., "user.get", "health") */
const procedurePathArb = fc.oneof(
  // Root-level procedure (no namespace)
  fc
    .string({ minLength: 1, maxLength: 20 })
    .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
  // Namespaced procedure (one level)
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
  // Namespaced procedure (two levels)
  fc
    .tuple(
      fc
        .string({ minLength: 1, maxLength: 8 })
        .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
      fc
        .string({ minLength: 1, maxLength: 8 })
        .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
      fc
        .string({ minLength: 1, maxLength: 8 })
        .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
    )
    .map(([ns1, ns2, name]) => `${ns1}.${ns2}.${name}`),
);

/** Generate a procedures record with unique paths */
const proceduresRecordArb = fc
  .array(fc.tuple(procedurePathArb, procedureSchemaArb), {
    minLength: 0,
    maxLength: 30,
  })
  .map((entries) => {
    const record: Record<string, ProcedureSchema> = {};
    const seenPaths = new Set<string>();
    for (const [path, schema] of entries) {
      if (!seenPaths.has(path)) {
        seenPaths.add(path);
        record[path] = schema;
      }
    }
    return record;
  });

/** Generate a filter type */
const filterTypeArb = fc.constantFrom<ProcedureType | "all">(
  "all",
  "query",
  "mutation",
  "subscription",
);

/** Generate a search string */
const searchStringArb = fc.oneof(
  fc.constant(""),
  fc.string({ minLength: 1, maxLength: 20 }),
);

// =============================================================================
// Property 1: Procedure Grouping Preserves All Procedures
// =============================================================================
// **Validates: Requirements 2.1**

describe("Property 1: Procedure Grouping Preserves All Procedures", () => {
  it("groupProcedures preserves total procedure count", () => {
    fc.assert(
      fc.property(proceduresRecordArb, (procedures) => {
        const groups = groupProcedures(procedures);
        const totalInGroups = groups.reduce(
          (sum, g) => sum + g.procedures.length,
          0,
        );
        const originalCount = Object.keys(procedures).length;

        expect(totalInGroups).toBe(originalCount);
      }),
      { numRuns: 100 },
    );
  });

  it("each procedure appears exactly once across all groups", () => {
    fc.assert(
      fc.property(proceduresRecordArb, (procedures) => {
        const groups = groupProcedures(procedures);
        const allPaths = groups.flatMap((g) => g.procedures.map((p) => p.path));
        const uniquePaths = new Set(allPaths);

        // No duplicates
        expect(allPaths.length).toBe(uniquePaths.size);
        // All original paths present
        expect(uniquePaths.size).toBe(Object.keys(procedures).length);
        for (const path of Object.keys(procedures)) {
          expect(uniquePaths.has(path)).toBe(true);
        }
      }),
      { numRuns: 100 },
    );
  });

  it("each procedure is in the correct namespace group", () => {
    fc.assert(
      fc.property(proceduresRecordArb, (procedures) => {
        const groups = groupProcedures(procedures);

        for (const group of groups) {
          for (const proc of group.procedures) {
            const expectedNamespace = proc.path.includes(".")
              ? proc.path.substring(0, proc.path.lastIndexOf("."))
              : "";
            expect(group.namespace).toBe(expectedNamespace);
          }
        }
      }),
      { numRuns: 100 },
    );
  });

  it("groups are sorted with root level first, then alphabetically", () => {
    fc.assert(
      fc.property(proceduresRecordArb, (procedures) => {
        const groups = groupProcedures(procedures);

        if (groups.length <= 1) return; // Nothing to check

        // Check root level is first if present
        const rootIndex = groups.findIndex((g) => g.namespace === "");
        if (rootIndex !== -1) {
          expect(rootIndex).toBe(0);
        }

        // Check non-root groups are sorted alphabetically
        const nonRootGroups = groups.filter((g) => g.namespace !== "");
        for (let i = 1; i < nonRootGroups.length; i++) {
          expect(
            nonRootGroups[i - 1].namespace.localeCompare(
              nonRootGroups[i].namespace,
            ),
          ).toBeLessThanOrEqual(0);
        }
      }),
      { numRuns: 100 },
    );
  });
});

// =============================================================================
// Property 5: Filtering Produces Correct Results
// =============================================================================
// **Validates: Requirements 4.1, 4.2**

describe("Property 5: Filtering Produces Correct Results", () => {
  it("text search filters by path (case-insensitive)", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        searchStringArb,
        (procedures, search) => {
          const result = filterProcedures(procedures, {
            search,
            typeFilter: "all",
          });

          if (search === "") {
            // Empty search returns all
            expect(result.count).toBe(Object.keys(procedures).length);
          } else {
            // All results must match search in path or description
            const lowerSearch = search.toLowerCase();
            for (const proc of result.procedures) {
              const pathMatch = proc.path.toLowerCase().includes(lowerSearch);
              const descMatch =
                proc.schema.description?.toLowerCase().includes(lowerSearch) ??
                false;
              expect(pathMatch || descMatch).toBe(true);
            }
          }
        },
      ),
      { numRuns: 100 },
    );
  });

  it("type filter returns only matching procedure types", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        filterTypeArb,
        (procedures, typeFilter) => {
          const result = filterProcedures(procedures, {
            search: "",
            typeFilter,
          });

          if (typeFilter === "all") {
            expect(result.count).toBe(Object.keys(procedures).length);
          } else {
            for (const proc of result.procedures) {
              expect(proc.schema.procedure_type).toBe(typeFilter);
            }
          }
        },
      ),
      { numRuns: 100 },
    );
  });

  it("combined filters apply AND logic", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        searchStringArb,
        filterTypeArb,
        (procedures, search, typeFilter) => {
          const result = filterProcedures(procedures, { search, typeFilter });

          // Verify each result matches both criteria
          const lowerSearch = search.toLowerCase();
          for (const proc of result.procedures) {
            // Type filter check
            if (typeFilter !== "all") {
              expect(proc.schema.procedure_type).toBe(typeFilter);
            }
            // Search check
            if (search !== "") {
              const pathMatch = proc.path.toLowerCase().includes(lowerSearch);
              const descMatch =
                proc.schema.description?.toLowerCase().includes(lowerSearch) ??
                false;
              expect(pathMatch || descMatch).toBe(true);
            }
          }
        },
      ),
      { numRuns: 100 },
    );
  });

  it("filtered results are a subset of original procedures", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        searchStringArb,
        filterTypeArb,
        (procedures, search, typeFilter) => {
          const result = filterProcedures(procedures, { search, typeFilter });

          expect(result.count).toBeLessThanOrEqual(
            Object.keys(procedures).length,
          );

          for (const proc of result.procedures) {
            expect(procedures[proc.path]).toBeDefined();
            expect(procedures[proc.path]).toBe(proc.schema);
          }
        },
      ),
      { numRuns: 100 },
    );
  });
});

// =============================================================================
// Property 6: Filter Count Accuracy
// =============================================================================
// **Validates: Requirements 4.4**

describe("Property 6: Filter Count Accuracy", () => {
  it("count matches actual filtered procedure length", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        searchStringArb,
        filterTypeArb,
        (procedures, search, typeFilter) => {
          const result = filterProcedures(procedures, { search, typeFilter });

          expect(result.count).toBe(result.procedures.length);
        },
      ),
      { numRuns: 100 },
    );
  });

  it("totalCount matches original procedure count", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        searchStringArb,
        filterTypeArb,
        (procedures, search, typeFilter) => {
          const result = filterProcedures(procedures, { search, typeFilter });

          expect(result.totalCount).toBe(Object.keys(procedures).length);
        },
      ),
      { numRuns: 100 },
    );
  });

  it("count is always less than or equal to totalCount", () => {
    fc.assert(
      fc.property(
        proceduresRecordArb,
        searchStringArb,
        filterTypeArb,
        (procedures, search, typeFilter) => {
          const result = filterProcedures(procedures, { search, typeFilter });

          expect(result.count).toBeLessThanOrEqual(result.totalCount);
        },
      ),
      { numRuns: 100 },
    );
  });
});

// =============================================================================
// Property 2: Placeholder Generation from Schema
// =============================================================================
// **Validates: Requirements 1.3**

import { generatePlaceholder, generatePlaceholderJson } from "../utils";
import type { TypeSchema } from "../types";

/** Generate a simple object schema with properties */
const objectSchemaArb: fc.Arbitrary<TypeSchema> = fc.record({
  type: fc.constant("object" as const),
  properties: fc.dictionary(
    fc
      .string({ minLength: 1, maxLength: 10 })
      .filter((s) => /^[a-z][a-z0-9_]*$/i.test(s)),
    fc.oneof(
      fc.record({ type: fc.constant("string" as const) }),
      fc.record({ type: fc.constant("number" as const) }),
      fc.record({ type: fc.constant("boolean" as const) }),
    ),
    { minKeys: 1, maxKeys: 5 },
  ),
});

describe("Property 2: Placeholder Generation from Schema", () => {
  it("returns empty object for undefined schema", () => {
    const result = generatePlaceholder(undefined);
    expect(result).toEqual({});
  });

  it("returns empty object for object schema without properties", () => {
    const result = generatePlaceholder({ type: "object" });
    expect(result).toEqual({});
  });

  it("object placeholder contains all property keys from schema", () => {
    fc.assert(
      fc.property(objectSchemaArb, (schema) => {
        const result = generatePlaceholder(schema);

        expect(typeof result).toBe("object");
        expect(result).not.toBeNull();

        const resultObj = result as Record<string, unknown>;
        const schemaKeys = Object.keys(schema.properties || {});
        const resultKeys = Object.keys(resultObj);

        // All schema keys should be in result
        for (const key of schemaKeys) {
          expect(resultKeys).toContain(key);
        }
        // Result should have same number of keys
        expect(resultKeys.length).toBe(schemaKeys.length);
      }),
      { numRuns: 100 },
    );
  });

  it("string type returns empty string by default", () => {
    const result = generatePlaceholder({ type: "string" });
    expect(result).toBe("");
  });

  it("number type returns 0 by default", () => {
    const result = generatePlaceholder({ type: "number" });
    expect(result).toBe(0);
  });

  it("integer type returns 0 by default", () => {
    const result = generatePlaceholder({ type: "integer" });
    expect(result).toBe(0);
  });

  it("boolean type returns false by default", () => {
    const result = generatePlaceholder({ type: "boolean" });
    expect(result).toBe(false);
  });

  it("null type returns null", () => {
    const result = generatePlaceholder({ type: "null" });
    expect(result).toBeNull();
  });

  it("array type returns array with one item placeholder", () => {
    const result = generatePlaceholder({
      type: "array",
      items: { type: "string" },
    });
    expect(Array.isArray(result)).toBe(true);
    expect((result as unknown[]).length).toBe(1);
    expect((result as unknown[])[0]).toBe("");
  });

  it("array without items returns empty array", () => {
    const result = generatePlaceholder({ type: "array" });
    expect(result).toEqual([]);
  });

  it("string with enum returns first enum value", () => {
    const result = generatePlaceholder({
      type: "string",
      enum: ["active", "inactive", "pending"],
    });
    expect(result).toBe("active");
  });

  it("generatePlaceholderJson returns valid JSON string", () => {
    fc.assert(
      fc.property(objectSchemaArb, (schema) => {
        const jsonStr = generatePlaceholderJson(schema);

        // Should be valid JSON
        expect(() => JSON.parse(jsonStr)).not.toThrow();

        // Parsed value should match generatePlaceholder
        const parsed = JSON.parse(jsonStr);
        const direct = generatePlaceholder(schema);
        expect(parsed).toEqual(direct);
      }),
      { numRuns: 100 },
    );
  });
});
