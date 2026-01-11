// =============================================================================
// Property-Based Tests for ProcedureTester Component
// =============================================================================
// Tests using fast-check to verify universal properties of ProcedureTester.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as fc from "fast-check";
import { render, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { ProcedureTester } from "../ProcedureTester";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

// =============================================================================
// Arbitraries (Generators)
// =============================================================================

/** Generate a valid procedure path */
const procedurePathArb = fc.oneof(
  fc
    .string({ minLength: 1, maxLength: 20 })
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

/** Generate valid JSON input */
const validJsonArb = fc.oneof(
  fc.constant("{}"),
  fc.constant('{"name": "test"}'),
  fc.constant('{"id": 123}'),
  fc.constant('{"active": true}'),
  fc
    .dictionary(
      fc
        .string({ minLength: 1, maxLength: 10 })
        .filter((s) => /^[a-z][a-z0-9]*$/i.test(s)),
      fc.oneof(fc.string(), fc.integer(), fc.boolean()),
      { minKeys: 0, maxKeys: 3 },
    )
    .map((obj) => JSON.stringify(obj)),
);

/** Generate invalid JSON strings */
const invalidJsonArb = fc.oneof(
  fc.constant("{invalid}"),
  fc.constant('{"unclosed": '),
  fc.constant('{key: "no quotes"}'),
  fc.constant("not json at all"),
  fc.constant("[1, 2, 3,]"), // trailing comma
);

/** Generate a response value */
const responseValueArb = fc.oneof(
  fc.string(),
  fc.integer(),
  fc.boolean(),
  fc.dictionary(fc.string({ minLength: 1, maxLength: 10 }), fc.string(), {
    maxKeys: 3,
  }),
);

// =============================================================================
// Property 3: RPC Call Execution
// =============================================================================
// **Validates: Requirements 2.1**

describe("Property 3: RPC Call Execution", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("invokes rpc_call with correct path and parsed input on execute", async () => {
    await fc.assert(
      fc.asyncProperty(
        procedurePathArb,
        validJsonArb,
        async (path, jsonInput) => {
          cleanup(); // Clean up before each iteration
          vi.mocked(invoke).mockResolvedValue({ success: true });

          const { getByTestId } = render(<ProcedureTester path={path} />);

          // Set input
          const textarea = getByTestId("input-editor-textarea");
          fireEvent.change(textarea, { target: { value: jsonInput } });

          // Click execute
          const executeBtn = getByTestId("procedure-tester-execute");
          fireEvent.click(executeBtn);

          await waitFor(() => {
            expect(invoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
              path,
              input: JSON.parse(jsonInput),
            });
          });
        },
      ),
      { numRuns: 50 },
    );
  });

  it("displays response after successful execution", async () => {
    await fc.assert(
      fc.asyncProperty(
        procedurePathArb,
        responseValueArb,
        async (path, response) => {
          cleanup(); // Clean up before each iteration
          vi.mocked(invoke).mockResolvedValue(response);

          const { getByTestId, queryByTestId } = render(
            <ProcedureTester path={path} />,
          );

          // Click execute
          const executeBtn = getByTestId("procedure-tester-execute");
          fireEvent.click(executeBtn);

          await waitFor(() => {
            const content = queryByTestId("response-viewer-content");
            expect(content).toBeTruthy();
            expect(content?.textContent).toBe(
              JSON.stringify(response, null, 2),
            );
          });
        },
      ),
      { numRuns: 30 },
    );
  });

  it("displays error after failed execution", async () => {
    await fc.assert(
      fc.asyncProperty(
        procedurePathArb,
        fc.string({ minLength: 1, maxLength: 100 }),
        async (path, errorMsg) => {
          cleanup(); // Clean up before each iteration
          vi.mocked(invoke).mockRejectedValue(new Error(errorMsg));

          const { getByTestId, queryByTestId } = render(
            <ProcedureTester path={path} />,
          );

          // Click execute
          const executeBtn = getByTestId("procedure-tester-execute");
          fireEvent.click(executeBtn);

          await waitFor(() => {
            const errorElement = queryByTestId("response-viewer-error");
            expect(errorElement).toBeTruthy();
            expect(errorElement?.textContent).toContain(errorMsg);
          });
        },
      ),
      { numRuns: 30 },
    );
  });
});

// =============================================================================
// Property 6: Invalid JSON Validation
// =============================================================================
// **Validates: Requirements 3.1, 3.2**

describe("Property 6: Invalid JSON Validation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("shows error and disables button for invalid JSON", () => {
    fc.assert(
      fc.property(procedurePathArb, invalidJsonArb, (path, invalidJson) => {
        cleanup(); // Clean up before each iteration
        const { getByTestId, queryByTestId } = render(
          <ProcedureTester path={path} />,
        );

        // Set invalid input
        const textarea = getByTestId("input-editor-textarea");
        fireEvent.change(textarea, { target: { value: invalidJson } });

        // Check error is shown
        const errorElement = queryByTestId("input-editor-error");
        expect(errorElement).toBeTruthy();

        // Check button is disabled
        const executeBtn = getByTestId("procedure-tester-execute");
        expect(executeBtn).toBeDisabled();
      }),
      { numRuns: 50 },
    );
  });

  it("clears error and enables button when JSON is fixed", () => {
    fc.assert(
      fc.property(
        procedurePathArb,
        invalidJsonArb,
        validJsonArb,
        (path, invalidJson, validJson) => {
          cleanup(); // Clean up before each iteration
          const { getByTestId, queryByTestId } = render(
            <ProcedureTester path={path} />,
          );

          const textarea = getByTestId("input-editor-textarea");

          // Set invalid input first
          fireEvent.change(textarea, { target: { value: invalidJson } });
          expect(queryByTestId("input-editor-error")).toBeTruthy();

          // Fix with valid JSON
          fireEvent.change(textarea, { target: { value: validJson } });

          // Error should be cleared
          expect(queryByTestId("input-editor-error")).toBeNull();

          // Button should be enabled
          const executeBtn = getByTestId("procedure-tester-execute");
          expect(executeBtn).not.toBeDisabled();
        },
      ),
      { numRuns: 50 },
    );
  });

  it("does not call invoke when JSON is invalid", () => {
    fc.assert(
      fc.property(procedurePathArb, invalidJsonArb, (path, invalidJson) => {
        cleanup(); // Clean up before each iteration
        vi.mocked(invoke).mockResolvedValue({});

        const { getByTestId } = render(<ProcedureTester path={path} />);

        // Set invalid input
        const textarea = getByTestId("input-editor-textarea");
        fireEvent.change(textarea, { target: { value: invalidJson } });

        // Try to click execute (should be disabled)
        const executeBtn = getByTestId("procedure-tester-execute");
        fireEvent.click(executeBtn);

        // invoke should not be called
        expect(invoke).not.toHaveBeenCalled();
      }),
      { numRuns: 30 },
    );
  });
});

// =============================================================================
// Placeholder Tests
// =============================================================================

describe("Placeholder Generation", () => {
  afterEach(() => {
    cleanup();
  });

  it("uses placeholder from inputSchema", () => {
    const inputSchema = {
      type: "object",
      properties: {
        name: { type: "string" },
        age: { type: "number" },
      },
    };

    const { getByTestId } = render(
      <ProcedureTester path="test.proc" inputSchema={inputSchema} />,
    );

    const textarea = getByTestId(
      "input-editor-textarea",
    ) as HTMLTextAreaElement;
    const value = textarea.value;

    // Should contain the property names
    expect(value).toContain("name");
    expect(value).toContain("age");
  });

  it("uses empty object placeholder when no schema", () => {
    const { getByTestId } = render(<ProcedureTester path="test.proc" />);

    const textarea = getByTestId(
      "input-editor-textarea",
    ) as HTMLTextAreaElement;
    expect(textarea.value).toBe("{}");
  });
});
