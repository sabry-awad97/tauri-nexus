import { describe, it, expect, vi } from "vitest";
import * as fc from "fast-check";
import { z } from "zod";
import {
  procedure,
  router,
  mergeRouters,
  ProcedureBuilder,
  createValidationInterceptor,
  buildSchemaMap,
  type SchemaContract,
} from "../schema";
import type { LinkRequestContext } from "../link";

// =============================================================================
// Test Helpers
// =============================================================================

/** Create a mock context for testing interceptors */
function createMockContext(
  path: string,
  input: unknown = null
): LinkRequestContext {
  return {
    path,
    input,
    type: "query",
    context: {},
    meta: {},
  };
}

/** Create a mock next function that returns the given value */
function createMockNext<T>(returnValue: T): () => Promise<T> {
  return vi.fn().mockResolvedValue(returnValue);
}

// =============================================================================
// Task 1: Core Types Tests
// =============================================================================

describe("Core Schema Types", () => {
  it("should create a procedure with input and output schemas", () => {
    const proc = procedure()
      .input(z.object({ id: z.number() }))
      .output(z.object({ name: z.string() }))
      .query();

    expect(proc.type).toBe("query");
    expect(proc.inputSchema).toBeDefined();
    expect(proc.outputSchema).toBeDefined();
  });

  it("should create a procedure without input schema (void input)", () => {
    const proc = procedure()
      .output(z.object({ status: z.string() }))
      .query();

    expect(proc.type).toBe("query");
    expect(proc.inputSchema).toBeNull();
    expect(proc.outputSchema).toBeDefined();
  });
});

// =============================================================================
// Task 2.1: ProcedureBuilder Tests
// =============================================================================

describe("ProcedureBuilder", () => {
  describe("fluent API", () => {
    it("should chain input() and output() methods", () => {
      const builder = new ProcedureBuilder()
        .input(z.string())
        .output(z.number());

      expect(builder).toBeInstanceOf(ProcedureBuilder);
    });

    it("should throw when calling query() without output schema", () => {
      const builder = new ProcedureBuilder().input(z.string());
      expect(() => builder.query()).toThrow(
        "Output schema is required"
      );
    });

    it("should throw when calling mutation() without output schema", () => {
      const builder = new ProcedureBuilder().input(z.string());
      expect(() => builder.mutation()).toThrow(
        "Output schema is required"
      );
    });

    it("should throw when calling subscription() without output schema", () => {
      const builder = new ProcedureBuilder().input(z.string());
      expect(() => builder.subscription()).toThrow(
        "Output schema is required"
      );
    });
  });
});

// =============================================================================
// Task 2.2: Property 1 - Contract Builder produces correct procedure types
// =============================================================================

describe("Property 1: Contract Builder produces correct procedure types", () => {
  it("query() produces type 'query'", () => {
    fc.assert(
      fc.property(fc.anything(), () => {
        const proc = procedure().output(z.string()).query();
        expect(proc.type).toBe("query");
      }),
      { numRuns: 100 }
    );
  });

  it("mutation() produces type 'mutation'", () => {
    fc.assert(
      fc.property(fc.anything(), () => {
        const proc = procedure().output(z.string()).mutation();
        expect(proc.type).toBe("mutation");
      }),
      { numRuns: 100 }
    );
  });

  it("subscription() produces type 'subscription'", () => {
    fc.assert(
      fc.property(fc.anything(), () => {
        const proc = procedure().output(z.string()).subscription();
        expect(proc.type).toBe("subscription");
      }),
      { numRuns: 100 }
    );
  });

  it("procedure type matches the terminal method called", () => {
    const types = ["query", "mutation", "subscription"] as const;

    fc.assert(
      fc.property(
        fc.constantFrom(...types),
        (procedureType) => {
          const builder = procedure().output(z.string());
          const proc = builder[procedureType]();
          expect(proc.type).toBe(procedureType);
        }
      ),
      { numRuns: 100 }
    );
  });
});

// =============================================================================
// Task 2.3: Router Utilities Tests
// =============================================================================

describe("Router Utilities", () => {
  it("router() creates a router from procedures", () => {
    const contract = router({
      health: procedure().output(z.object({ status: z.string() })).query(),
    });

    expect(contract.health).toBeDefined();
    expect(contract.health.type).toBe("query");
  });

  it("router() supports nested routers", () => {
    const contract = router({
      user: router({
        get: procedure()
          .input(z.object({ id: z.number() }))
          .output(z.object({ name: z.string() }))
          .query(),
      }),
    });

    expect(contract.user.get).toBeDefined();
    expect(contract.user.get.type).toBe("query");
  });
});

// =============================================================================
// Task 2.4: Property 2 - Nested router structures are correctly built
// =============================================================================

describe("Property 2: Nested router structures are correctly built", () => {
  it("all procedures at any depth are accessible via buildSchemaMap", () => {
    const contract = router({
      health: procedure().output(z.string()).query(),
      user: router({
        get: procedure().input(z.number()).output(z.string()).query(),
        profile: router({
          update: procedure().input(z.string()).output(z.boolean()).mutation(),
        }),
      }),
    });

    const schemaMap = buildSchemaMap(contract);

    expect(schemaMap.has("health")).toBe(true);
    expect(schemaMap.has("user.get")).toBe(true);
    expect(schemaMap.has("user.profile.update")).toBe(true);
    expect(schemaMap.get("health")?.type).toBe("query");
    expect(schemaMap.get("user.get")?.type).toBe("query");
    expect(schemaMap.get("user.profile.update")?.type).toBe("mutation");
  });

  it("schemas are preserved at all depths", () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 5 }),
        (depth) => {
          // Build a nested router of given depth
          let innerRouter: SchemaContract = {
            leaf: procedure().input(z.number()).output(z.string()).query(),
          };

          for (let i = 0; i < depth; i++) {
            innerRouter = router({ nested: innerRouter });
          }

          const schemaMap = buildSchemaMap(innerRouter);
          const expectedPath = "nested.".repeat(depth) + "leaf";

          expect(schemaMap.has(expectedPath)).toBe(true);
          expect(schemaMap.get(expectedPath)?.inputSchema).toBeDefined();
          expect(schemaMap.get(expectedPath)?.outputSchema).toBeDefined();
        }
      ),
      { numRuns: 100 }
    );
  });
});

// =============================================================================
// Task 2.4: Property 10 - Router merge combines all procedures
// =============================================================================

describe("Property 10: Router merge combines all procedures", () => {
  it("mergeRouters combines all procedures from both routers", () => {
    const router1 = router({
      health: procedure().output(z.string()).query(),
    });

    const router2 = router({
      user: router({
        get: procedure().output(z.string()).query(),
      }),
    });

    const merged = mergeRouters(router1, router2);
    const schemaMap = buildSchemaMap(merged);

    expect(schemaMap.has("health")).toBe(true);
    expect(schemaMap.has("user.get")).toBe(true);
  });

  it("later routers override earlier ones on key conflicts", () => {
    const router1 = router({
      health: procedure().output(z.string()).query(),
    });

    const router2 = router({
      health: procedure().output(z.number()).mutation(),
    });

    const merged = mergeRouters(router1, router2);

    expect(merged.health.type).toBe("mutation");
  });

  it("merge is associative for non-conflicting keys", () => {
    fc.assert(
      fc.property(
        fc.array(fc.string().filter(s => /^[a-z]+$/.test(s)), { minLength: 1, maxLength: 5 }),
        (keys) => {
          const uniqueKeys = [...new Set(keys)];
          const routers = uniqueKeys.map(key =>
            router({ [key]: procedure().output(z.string()).query() })
          );

          if (routers.length === 0) return;

          const merged = mergeRouters(...routers);
          const schemaMap = buildSchemaMap(merged);

          for (const key of uniqueKeys) {
            expect(schemaMap.has(key)).toBe(true);
          }
        }
      ),
      { numRuns: 100 }
    );
  });
});


// =============================================================================
// Task 4.1-4.2: Schema Map and Input Validation Tests
// =============================================================================

describe("Schema Map Builder", () => {
  it("builds correct map from flat contract", () => {
    const contract = router({
      health: procedure().output(z.string()).query(),
      create: procedure().input(z.number()).output(z.string()).mutation(),
    });

    const map = buildSchemaMap(contract);

    expect(map.size).toBe(2);
    expect(map.get("health")?.inputSchema).toBeNull();
    expect(map.get("create")?.inputSchema).toBeDefined();
  });

  it("builds correct map from nested contract", () => {
    const contract = router({
      user: router({
        get: procedure().input(z.number()).output(z.string()).query(),
        create: procedure().input(z.string()).output(z.number()).mutation(),
      }),
    });

    const map = buildSchemaMap(contract);

    expect(map.size).toBe(2);
    expect(map.has("user.get")).toBe(true);
    expect(map.has("user.create")).toBe(true);
  });
});

// =============================================================================
// Task 4.3: Property 3 - Valid input passes validation
// =============================================================================

describe("Property 3: Valid input passes validation", () => {
  it("valid input passes through without throwing", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ name: z.string(), age: z.number() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", { name: "John", age: 30 });
    const next = createMockNext("success");

    const result = await interceptor(ctx, next);

    expect(result).toBe("success");
    expect(next).toHaveBeenCalled();
  });

  it("valid input with various types passes validation", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.record({
          str: fc.string(),
          num: fc.integer(),
          bool: fc.boolean(),
        }),
        async (input) => {
          const contract = router({
            test: procedure()
              .input(z.object({
                str: z.string(),
                num: z.number(),
                bool: z.boolean(),
              }))
              .output(z.string())
              .query(),
          });

          const interceptor = createValidationInterceptor(contract);
          const ctx = createMockContext("test", input);
          const next = createMockNext("ok");

          const result = await interceptor(ctx, next);
          expect(result).toBe("ok");
        }
      ),
      { numRuns: 100 }
    );
  });
});

// =============================================================================
// Task 4.3: Property 4 - Invalid input produces VALIDATION_ERROR
// =============================================================================

describe("Property 4: Invalid input produces VALIDATION_ERROR with Zod details", () => {
  it("invalid input throws VALIDATION_ERROR", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ name: z.string() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", { name: 123 }); // wrong type
    const next = createMockNext("success");

    await expect(interceptor(ctx, next)).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
      message: expect.stringContaining("Input validation failed"),
    });
  });

  it("validation error includes Zod issue details", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ email: z.string().email() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", { email: "not-an-email" });
    const next = createMockNext("success");

    try {
      await interceptor(ctx, next);
      expect.fail("Should have thrown");
    } catch (error: unknown) {
      const rpcError = error as { code: string; details: { issues: Array<{ path: string; message: string }> } };
      expect(rpcError.code).toBe("VALIDATION_ERROR");
      expect(rpcError.details.issues).toBeInstanceOf(Array);
      expect(rpcError.details.issues.length).toBeGreaterThan(0);
    }
  });

  it("invalid input with random wrong types throws VALIDATION_ERROR", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.oneof(fc.string(), fc.boolean(), fc.array(fc.anything())),
        async (wrongInput) => {
          const contract = router({
            test: procedure()
              .input(z.object({ id: z.number() }))
              .output(z.string())
              .query(),
          });

          const interceptor = createValidationInterceptor(contract);
          const ctx = createMockContext("test", wrongInput);
          const next = createMockNext("ok");

          await expect(interceptor(ctx, next)).rejects.toMatchObject({
            code: "VALIDATION_ERROR",
          });
        }
      ),
      { numRuns: 100 }
    );
  });
});

// =============================================================================
// Task 4.4-4.5: Output Validation Tests
// =============================================================================

describe("Property 6: Valid output passes validation", () => {
  it("valid output passes through without throwing", async () => {
    const contract = router({
      test: procedure()
        .output(z.object({ id: z.number(), name: z.string() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test");
    const next = createMockNext({ id: 1, name: "Test" });

    const result = await interceptor(ctx, next);

    expect(result).toEqual({ id: 1, name: "Test" });
  });

  it("valid output with various types passes validation", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.record({
          id: fc.integer(),
          name: fc.string(),
          active: fc.boolean(),
        }),
        async (output) => {
          const contract = router({
            test: procedure()
              .output(z.object({
                id: z.number(),
                name: z.string(),
                active: z.boolean(),
              }))
              .query(),
          });

          const interceptor = createValidationInterceptor(contract);
          const ctx = createMockContext("test");
          const next = createMockNext(output);

          const result = await interceptor(ctx, next);
          expect(result).toEqual(output);
        }
      ),
      { numRuns: 100 }
    );
  });
});

describe("Property 7: Invalid output produces VALIDATION_ERROR with details", () => {
  it("invalid output throws VALIDATION_ERROR", async () => {
    const contract = router({
      test: procedure()
        .output(z.object({ id: z.number() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test");
    const next = createMockNext({ id: "not-a-number" }); // wrong type

    await expect(interceptor(ctx, next)).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
      message: expect.stringContaining("Output validation failed"),
    });
  });

  it("output validation error includes details", async () => {
    const contract = router({
      test: procedure()
        .output(z.object({ count: z.number().min(0) }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test");
    const next = createMockNext({ count: -1 });

    try {
      await interceptor(ctx, next);
      expect.fail("Should have thrown");
    } catch (error: unknown) {
      const rpcError = error as { code: string; details: { type: string; issues: unknown[] } };
      expect(rpcError.code).toBe("VALIDATION_ERROR");
      expect(rpcError.details.type).toBe("output");
      expect(rpcError.details.issues.length).toBeGreaterThan(0);
    }
  });
});

// =============================================================================
// Task 4.6: Property 5 - Zod transforms are applied correctly
// =============================================================================

describe("Property 5: Zod transforms are applied correctly", () => {
  it("input transforms are applied before passing to next", async () => {
    const contract = router({
      test: procedure()
        .input(z.string().transform(s => s.toUpperCase()))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", "hello");
    const next = createMockNext("ok");

    await interceptor(ctx, next);

    // The context input should be transformed
    expect(ctx.input).toBe("HELLO");
  });

  it("output transforms are applied to response", async () => {
    const contract = router({
      test: procedure()
        .output(z.string().transform(s => s.toLowerCase()))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test");
    const next = createMockNext("HELLO");

    const result = await interceptor(ctx, next);

    expect(result).toBe("hello");
  });

  it("date transforms work correctly", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({
          date: z.string().transform(s => new Date(s)),
        }))
        .output(z.object({
          timestamp: z.number(),
        }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", { date: "2024-01-01" });
    const next = createMockNext({ timestamp: 1704067200000 });

    await interceptor(ctx, next);

    expect((ctx.input as { date: Date }).date).toBeInstanceOf(Date);
  });

  it("chained transforms are applied in order", async () => {
    const contract = router({
      test: procedure()
        .input(
          z.string()
            .transform(s => s.trim())
            .transform(s => s.toUpperCase())
            .transform(s => `[${s}]`)
        )
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", "  hello  ");
    const next = createMockNext("ok");

    await interceptor(ctx, next);

    expect(ctx.input).toBe("[HELLO]");
  });
});


// =============================================================================
// Task 6: Validation Configuration Tests
// =============================================================================

describe("Property 8: Disabled validation passes data unchanged", () => {
  it("disabled input validation passes invalid input through", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ id: z.number() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      validateInput: false,
    });
    const ctx = createMockContext("test", { id: "not-a-number" });
    const next = createMockNext("ok");

    const result = await interceptor(ctx, next);

    expect(result).toBe("ok");
    expect(ctx.input).toEqual({ id: "not-a-number" }); // unchanged
  });

  it("disabled output validation passes invalid output through", async () => {
    const contract = router({
      test: procedure()
        .output(z.object({ id: z.number() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      validateOutput: false,
    });
    const ctx = createMockContext("test");
    const next = createMockNext({ id: "not-a-number" });

    const result = await interceptor(ctx, next);

    expect(result).toEqual({ id: "not-a-number" }); // unchanged
  });

  it("both validations disabled passes everything through", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.anything(),
        fc.anything(),
        async (input, output) => {
          const contract = router({
            test: procedure()
              .input(z.object({ strict: z.literal(true) }))
              .output(z.object({ strict: z.literal(true) }))
              .query(),
          });

          const interceptor = createValidationInterceptor(contract, {
            validateInput: false,
            validateOutput: false,
          });
          const ctx = createMockContext("test", input);
          const next = createMockNext(output);

          const result = await interceptor(ctx, next);

          expect(result).toEqual(output);
        }
      ),
      { numRuns: 100 }
    );
  });
});

describe("Property 9: Strict mode rejects unknown keys", () => {
  it("strict mode rejects input with unknown keys", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ id: z.number() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      strict: true,
    });
    const ctx = createMockContext("test", { id: 1, extra: "unknown" });
    const next = createMockNext("ok");

    await expect(interceptor(ctx, next)).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
    });
  });

  it("strict mode rejects output with unknown keys", async () => {
    const contract = router({
      test: procedure()
        .output(z.object({ id: z.number() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      strict: true,
    });
    const ctx = createMockContext("test");
    const next = createMockNext({ id: 1, extra: "unknown" });

    await expect(interceptor(ctx, next)).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
    });
  });

  it("non-strict mode allows unknown keys (strips them by default)", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ id: z.number() }))
        .output(z.object({ id: z.number() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      strict: false,
    });
    const ctx = createMockContext("test", { id: 1, extra: "allowed" });
    const next = createMockNext({ id: 1, extra: "allowed" });

    // Non-strict mode doesn't throw, but Zod strips unknown keys by default
    const result = await interceptor(ctx, next);

    // Zod strips unknown keys in non-strict mode
    expect(result).toEqual({ id: 1 });
  });

  it("strict mode with random extra keys always rejects", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.string().filter(s => s !== "id" && /^[a-z]+$/.test(s)),
        fc.anything(),
        async (extraKey, extraValue) => {
          const contract = router({
            test: procedure()
              .input(z.object({ id: z.number() }))
              .output(z.string())
              .query(),
          });

          const interceptor = createValidationInterceptor(contract, {
            strict: true,
          });
          const ctx = createMockContext("test", { id: 1, [extraKey]: extraValue });
          const next = createMockNext("ok");

          await expect(interceptor(ctx, next)).rejects.toMatchObject({
            code: "VALIDATION_ERROR",
          });
        }
      ),
      { numRuns: 100 }
    );
  });
});

describe("Custom error handler", () => {
  it("calls custom error handler on input validation failure", async () => {
    const onValidationError = vi.fn();

    const contract = router({
      test: procedure()
        .input(z.object({ id: z.number() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      onValidationError,
    });
    const ctx = createMockContext("test", { id: "wrong" });
    const next = createMockNext("ok");

    await expect(interceptor(ctx, next)).rejects.toThrow();

    expect(onValidationError).toHaveBeenCalledWith(
      expect.any(z.ZodError),
      { path: "test", type: "input" }
    );
  });

  it("calls custom error handler on output validation failure", async () => {
    const onValidationError = vi.fn();

    const contract = router({
      test: procedure()
        .output(z.object({ id: z.number() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract, {
      onValidationError,
    });
    const ctx = createMockContext("test");
    const next = createMockNext({ id: "wrong" });

    await expect(interceptor(ctx, next)).rejects.toThrow();

    expect(onValidationError).toHaveBeenCalledWith(
      expect.any(z.ZodError),
      { path: "test", type: "output" }
    );
  });
});

// =============================================================================
// Task 7: Void Input Procedures
// =============================================================================

describe("Void input procedures", () => {
  it("skips input validation for void input procedures", async () => {
    const contract = router({
      health: procedure()
        .output(z.object({ status: z.string() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("health", null);
    const next = createMockNext({ status: "ok" });

    const result = await interceptor(ctx, next);

    expect(result).toEqual({ status: "ok" });
  });

  it("skips input validation when input is undefined", async () => {
    const contract = router({
      health: procedure()
        .output(z.object({ status: z.string() }))
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("health", undefined);
    const next = createMockNext({ status: "ok" });

    const result = await interceptor(ctx, next);

    expect(result).toEqual({ status: "ok" });
  });
});

// =============================================================================
// Task 7.2: Integration Tests
// =============================================================================

describe("Integration: End-to-end validation flow", () => {
  it("validates both input and output in sequence", async () => {
    const contract = router({
      user: router({
        create: procedure()
          .input(z.object({
            name: z.string().min(1),
            email: z.string().email(),
          }))
          .output(z.object({
            id: z.number(),
            name: z.string(),
            email: z.string(),
          }))
          .mutation(),
      }),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("user.create", {
      name: "John",
      email: "john@example.com",
    });
    const next = createMockNext({
      id: 1,
      name: "John",
      email: "john@example.com",
    });

    const result = await interceptor(ctx, next);

    expect(result).toEqual({
      id: 1,
      name: "John",
      email: "john@example.com",
    });
  });

  it("fails fast on input validation before calling backend", async () => {
    const contract = router({
      test: procedure()
        .input(z.object({ id: z.number() }))
        .output(z.string())
        .query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("test", { id: "wrong" });
    const next = vi.fn().mockResolvedValue("ok");

    await expect(interceptor(ctx, next)).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
    });

    // Backend should not be called
    expect(next).not.toHaveBeenCalled();
  });

  it("handles unknown paths gracefully (no validation)", async () => {
    const contract = router({
      known: procedure().output(z.string()).query(),
    });

    const interceptor = createValidationInterceptor(contract);
    const ctx = createMockContext("unknown.path", { any: "data" });
    const next = createMockNext({ any: "response" });

    // Should pass through without validation
    const result = await interceptor(ctx, next);
    expect(result).toEqual({ any: "response" });
  });
});
