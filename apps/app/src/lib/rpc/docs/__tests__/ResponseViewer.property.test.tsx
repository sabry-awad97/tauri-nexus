// =============================================================================
// Property-Based Tests for ResponseViewer Component
// =============================================================================
// Tests using fast-check to verify universal properties of ResponseViewer.

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import { render } from "@testing-library/react";
import { ResponseViewer } from "../ResponseViewer";

// =============================================================================
// Arbitraries (Generators)
// =============================================================================

/** Generate a JSON-serializable response value (non-null for success case) */
const responseValueArb = fc.oneof(
  fc.string(),
  fc.integer(),
  fc.boolean(),
  fc.array(fc.string(), { maxLength: 5 }),
  fc.dictionary(fc.string({ minLength: 1, maxLength: 10 }), fc.string(), {
    maxKeys: 5,
  }),
);

/** Generate an error message */
const errorMessageArb = fc.string({ minLength: 1, maxLength: 200 });

/** Generate execution time in ms */
const executionTimeArb = fc.integer({ min: 0, max: 10000 });

// =============================================================================
// Property 4: Success Response Display
// =============================================================================
// **Validates: Requirements 2.3**

describe("Property 4: Success Response Display", () => {
  it("displays JSON-stringified response data on success", () => {
    fc.assert(
      fc.property(responseValueArb, executionTimeArb, (response, time) => {
        const { container } = render(
          <ResponseViewer
            response={response}
            error={null}
            executionTime={time}
            isLoading={false}
          />,
        );

        const content = container.querySelector(
          '[data-testid="response-viewer-content"]',
        );
        expect(content).toBeTruthy();

        // Content should contain the JSON-stringified response
        const expectedJson = JSON.stringify(response, null, 2);
        expect(content?.textContent).toBe(expectedJson);
      }),
      { numRuns: 100 },
    );
  });

  it("does not show error element on success", () => {
    fc.assert(
      fc.property(responseValueArb, (response) => {
        const { container } = render(
          <ResponseViewer
            response={response}
            error={null}
            executionTime={100}
            isLoading={false}
          />,
        );

        const errorElement = container.querySelector(
          '[data-testid="response-viewer-error"]',
        );
        expect(errorElement).toBeNull();
      }),
      { numRuns: 50 },
    );
  });
});

// =============================================================================
// Property 5: Error Response Display
// =============================================================================
// **Validates: Requirements 2.4**

describe("Property 5: Error Response Display", () => {
  it("displays error message with error styling on failure", () => {
    fc.assert(
      fc.property(errorMessageArb, executionTimeArb, (errorMsg, time) => {
        const { container } = render(
          <ResponseViewer
            response={null}
            error={errorMsg}
            executionTime={time}
            isLoading={false}
          />,
        );

        const errorElement = container.querySelector(
          '[data-testid="response-viewer-error"]',
        );
        expect(errorElement).toBeTruthy();
        expect(errorElement?.textContent).toContain(errorMsg);
      }),
      { numRuns: 100 },
    );
  });

  it("does not show response content on error", () => {
    fc.assert(
      fc.property(errorMessageArb, (errorMsg) => {
        const { container } = render(
          <ResponseViewer
            response={null}
            error={errorMsg}
            executionTime={100}
            isLoading={false}
          />,
        );

        const content = container.querySelector(
          '[data-testid="response-viewer-content"]',
        );
        expect(content).toBeNull();
      }),
      { numRuns: 50 },
    );
  });
});

// =============================================================================
// Property 7: Execution Time Display
// =============================================================================
// **Validates: Requirements 4.3**

describe("Property 7: Execution Time Display", () => {
  it("displays execution time in milliseconds for successful responses", () => {
    fc.assert(
      fc.property(responseValueArb, executionTimeArb, (response, time) => {
        const { container } = render(
          <ResponseViewer
            response={response}
            error={null}
            executionTime={time}
            isLoading={false}
          />,
        );

        const timeElement = container.querySelector(
          '[data-testid="response-viewer-time"]',
        );
        expect(timeElement).toBeTruthy();
        expect(timeElement?.textContent).toBe(`${time}ms`);
      }),
      { numRuns: 100 },
    );
  });

  it("displays execution time in milliseconds for error responses", () => {
    fc.assert(
      fc.property(errorMessageArb, executionTimeArb, (errorMsg, time) => {
        const { container } = render(
          <ResponseViewer
            response={null}
            error={errorMsg}
            executionTime={time}
            isLoading={false}
          />,
        );

        const timeElement = container.querySelector(
          '[data-testid="response-viewer-time"]',
        );
        expect(timeElement).toBeTruthy();
        expect(timeElement?.textContent).toBe(`${time}ms`);
      }),
      { numRuns: 100 },
    );
  });

  it("does not display time when executionTime is null", () => {
    const { container } = render(
      <ResponseViewer
        response={{ test: "data" }}
        error={null}
        executionTime={null}
        isLoading={false}
      />,
    );

    const timeElement = container.querySelector(
      '[data-testid="response-viewer-time"]',
    );
    expect(timeElement).toBeNull();
  });
});

// =============================================================================
// Loading State Tests
// =============================================================================

describe("Loading State", () => {
  it("shows loading indicator when isLoading is true", () => {
    const { container } = render(
      <ResponseViewer
        response={null}
        error={null}
        executionTime={null}
        isLoading={true}
      />,
    );

    const loading = container.querySelector(
      '[data-testid="response-viewer-loading"]',
    );
    expect(loading).toBeTruthy();
  });

  it("hides response content while loading", () => {
    const { container } = render(
      <ResponseViewer
        response={{ test: "data" }}
        error={null}
        executionTime={100}
        isLoading={true}
      />,
    );

    const content = container.querySelector(
      '[data-testid="response-viewer-content"]',
    );
    expect(content).toBeNull();
  });
});

// =============================================================================
// Empty State Tests
// =============================================================================

describe("Empty State", () => {
  it("renders nothing when no response, error, or loading", () => {
    const { container } = render(
      <ResponseViewer
        response={null}
        error={null}
        executionTime={null}
        isLoading={false}
      />,
    );

    const viewer = container.querySelector('[data-testid="response-viewer"]');
    expect(viewer).toBeNull();
  });
});
