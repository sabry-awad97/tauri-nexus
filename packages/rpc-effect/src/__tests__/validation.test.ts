// =============================================================================
// Validation Tests
// =============================================================================

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import { Effect, pipe } from "effect";
import {
  validatePath,
  validatePaths,
  isValidPath,
  validatePathWithRules,
} from "../validation";

describe("validatePath", () => {
  it("should accept valid paths", async () => {
    const validPaths = [
      "health",
      "user.get",
      "api.v1.users.list",
      "a_b_c",
      "test123",
      "User.Get",
    ];

    for (const path of validPaths) {
      const result = await Effect.runPromise(validatePath(path));
      expect(result).toBe(path);
    }
  });

  it("should reject empty paths", async () => {
    const result = await Effect.runPromise(
      pipe(validatePath(""), Effect.either),
    );
    expect(result._tag).toBe("Left");
    if (result._tag === "Left") {
      expect(result.left._tag).toBe("RpcValidationError");
    }
  });

  it("should reject paths starting with dot", async () => {
    const result = await Effect.runPromise(
      pipe(validatePath(".path"), Effect.either),
    );
    expect(result._tag).toBe("Left");
  });

  it("should reject paths ending with dot", async () => {
    const result = await Effect.runPromise(
      pipe(validatePath("path."), Effect.either),
    );
    expect(result._tag).toBe("Left");
  });

  it("should reject paths with consecutive dots", async () => {
    const result = await Effect.runPromise(
      pipe(validatePath("path..name"), Effect.either),
    );
    expect(result._tag).toBe("Left");
  });

  it("should reject paths with invalid characters", async () => {
    const invalidPaths = ["path/name", "path name", "path-name", "path@name"];

    for (const path of invalidPaths) {
      const result = await Effect.runPromise(
        pipe(validatePath(path), Effect.either),
      );
      expect(result._tag).toBe("Left");
    }
  });
});

describe("validatePaths", () => {
  it("should validate multiple valid paths", async () => {
    const paths = ["health", "user.get", "api.list"];
    const result = await Effect.runPromise(validatePaths(paths));
    expect(result).toEqual(paths);
  });

  it("should collect errors from multiple invalid paths", async () => {
    const paths = ["valid", "", ".invalid"];
    const result = await Effect.runPromise(
      pipe(validatePaths(paths), Effect.either),
    );
    expect(result._tag).toBe("Left");
  });
});

describe("isValidPath", () => {
  it("should return true for valid paths", async () => {
    const result = await Effect.runPromise(isValidPath("user.get"));
    expect(result).toBe(true);
  });

  it("should return false for invalid paths", async () => {
    const result = await Effect.runPromise(isValidPath(""));
    expect(result).toBe(false);
  });
});

describe("validatePathWithRules", () => {
  it("should enforce maxLength", async () => {
    const result = await Effect.runPromise(
      pipe(
        validatePathWithRules("very.long.path.name", { maxLength: 10 }),
        Effect.either,
      ),
    );
    expect(result._tag).toBe("Left");
  });

  it("should enforce minSegments", async () => {
    const result = await Effect.runPromise(
      pipe(validatePathWithRules("single", { minSegments: 2 }), Effect.either),
    );
    expect(result._tag).toBe("Left");
  });

  it("should enforce maxSegments", async () => {
    const result = await Effect.runPromise(
      pipe(
        validatePathWithRules("a.b.c.d.e", { maxSegments: 3 }),
        Effect.either,
      ),
    );
    expect(result._tag).toBe("Left");
  });

  it("should enforce allowedPrefixes", async () => {
    const result = await Effect.runPromise(
      pipe(
        validatePathWithRules("other.path", {
          allowedPrefixes: ["api.", "v1."],
        }),
        Effect.either,
      ),
    );
    expect(result._tag).toBe("Left");
  });

  it("should enforce disallowedPrefixes", async () => {
    const result = await Effect.runPromise(
      pipe(
        validatePathWithRules("internal.secret", {
          disallowedPrefixes: ["internal."],
        }),
        Effect.either,
      ),
    );
    expect(result._tag).toBe("Left");
  });

  it("should pass with valid rules", async () => {
    const result = await Effect.runPromise(
      validatePathWithRules("api.users.list", {
        maxLength: 50,
        minSegments: 2,
        maxSegments: 5,
        allowedPrefixes: ["api."],
      }),
    );
    expect(result).toBe("api.users.list");
  });
});

describe("Property-Based Tests", () => {
  it("property: valid paths always pass validation", () => {
    const validSegmentArb = fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/);
    const validPathArb = fc
      .array(validSegmentArb, { minLength: 1, maxLength: 5 })
      .map((segments) => segments.join("."));

    fc.assert(
      fc.asyncProperty(validPathArb, async (path) => {
        const result = await Effect.runPromise(isValidPath(path));
        expect(result).toBe(true);
      }),
      { numRuns: 100 },
    );
  });

  it("property: invalid paths are rejected", () => {
    const invalidPathArb = fc.oneof(
      fc.constant(""),
      fc
        .string({ minLength: 1 })
        .map((s) => "." + s.replace(/[^a-zA-Z0-9_.]/g, "")),
      fc
        .string({ minLength: 1 })
        .map((s) => s.replace(/[^a-zA-Z0-9_.]/g, "") + "."),
      fc
        .tuple(
          fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/),
          fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/),
        )
        .map(([a, b]) => `${a}..${b}`),
    );

    fc.assert(
      fc.asyncProperty(invalidPathArb, async (path) => {
        const result = await Effect.runPromise(isValidPath(path));
        expect(result).toBe(false);
      }),
      { numRuns: 100 },
    );
  });
});
