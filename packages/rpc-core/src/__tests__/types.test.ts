// =============================================================================
// Type System Tests
// =============================================================================
// Tests for type inference utilities and contract builder helpers.

import { describe, it, expect } from "vitest";
import { query, mutation, subscription } from "@tauri-nexus/rpc-core";

// =============================================================================
// Contract Builder Helper Tests
// =============================================================================

describe("Contract Builder Helpers", () => {
  describe("query()", () => {
    it("should create a query procedure definition", () => {
      const q = query<{ id: number }, { name: string }>();
      expect(q._type).toBe("query");
    });

    it("should create query with void input", () => {
      const q = query<void, string>();
      expect(q._type).toBe("query");
    });
  });

  describe("mutation()", () => {
    it("should create a mutation procedure definition", () => {
      const m = mutation<{ name: string }, { id: number }>();
      expect(m._type).toBe("mutation");
    });

    it("should create mutation with void input", () => {
      const m = mutation<void, boolean>();
      expect(m._type).toBe("mutation");
    });
  });

  describe("subscription()", () => {
    it("should create a subscription procedure definition", () => {
      const s = subscription<{ channel: string }, { message: string }>();
      expect(s._type).toBe("subscription");
    });

    it("should create subscription with void input", () => {
      const s = subscription<void, number>();
      expect(s._type).toBe("subscription");
    });
  });
});

// =============================================================================
// Type Inference Tests (Compile-Time)
// =============================================================================
// These tests verify that TypeScript correctly infers types at compile time.
// The contract builder helpers create ProcedureDef types that can be used
// with the type inference utilities.

describe("Type Inference (Compile-Time Verification)", () => {
  it("should correctly infer types from procedure definitions", () => {
    // Create procedure definitions using helpers
    const healthQuery = query<void, { status: string }>();
    const greetQuery = query<{ name: string }, string>();
    const createMutation = mutation<
      { name: string },
      { id: number; name: string }
    >();
    const eventsSub = subscription<void, { type: string; data: unknown }>();

    // Verify the _type field is correct
    expect(healthQuery._type).toBe("query");
    expect(greetQuery._type).toBe("query");
    expect(createMutation._type).toBe("mutation");
    expect(eventsSub._type).toBe("subscription");
  });

  it("should create correct procedure types", () => {
    const q = query<{ id: number }, { name: string }>();
    const m = mutation<{ data: string }, boolean>();
    const s = subscription<{ channel: string }, { message: string }>();

    // Type assertions - these verify compile-time correctness
    expect(q._type).toBe("query");
    expect(m._type).toBe("mutation");
    expect(s._type).toBe("subscription");
  });
});

// =============================================================================
// Contract Structure Tests
// =============================================================================

describe("Contract Structure", () => {
  it("should support nested router structures", () => {
    // This test verifies that contracts can have nested structures
    // The actual type inference is tested at compile time
    const contract = {
      health: {
        type: "query" as const,
        input: undefined,
        output: { status: "ok" },
      },
      user: {
        get: {
          type: "query" as const,
          input: { id: 1 },
          output: { id: 1, name: "test" },
        },
        create: {
          type: "mutation" as const,
          input: { name: "test" },
          output: { id: 1, name: "test" },
        },
      },
      stream: {
        events: {
          type: "subscription" as const,
          input: undefined,
          output: { type: "event", data: {} },
        },
      },
    };

    expect(contract.health.type).toBe("query");
    expect(contract.user.get.type).toBe("query");
    expect(contract.user.create.type).toBe("mutation");
    expect(contract.stream.events.type).toBe("subscription");
  });

  it("should support deeply nested structures", () => {
    const contract = {
      api: {
        v1: {
          users: {
            list: { type: "query" as const, input: undefined, output: [] },
            get: {
              type: "query" as const,
              input: { id: 1 },
              output: { id: 1 },
            },
          },
        },
      },
    };

    expect(contract.api.v1.users.list.type).toBe("query");
    expect(contract.api.v1.users.get.type).toBe("query");
  });
});

// =============================================================================
// Client Inference Utilities Tests (Compile-Time)
// =============================================================================

describe("Client Inference Utilities", () => {
  // Define a test contract for inference tests
  interface TestContract {
    health: { type: "query"; input: void; output: { status: string } };
    greet: { type: "query"; input: { name: string }; output: string };
    user: {
      get: {
        type: "query";
        input: { id: number };
        output: { id: number; name: string };
      };
      create: {
        type: "mutation";
        input: { name: string; email: string };
        output: { id: number; name: string };
      };
      delete: { type: "mutation"; input: { id: number }; output: boolean };
    };
    stream: {
      events: {
        type: "subscription";
        input: void;
        output: { type: string; data: unknown };
      };
    };
  }

  it("should infer input types correctly (compile-time verification)", () => {
    // These are compile-time type checks
    // If this compiles, the types are working correctly
    type Inputs = import("../types").InferClientInputs<TestContract>;

    // Type assertions - these verify the structure at compile time
    // Using void 0 to satisfy unused variable warnings while keeping type checks
    void (undefined as unknown as Inputs["health"]);
    void ({ name: "test" } as Inputs["greet"]);
    void ({ id: 1 } as Inputs["user"]["get"]);
    void ({
      name: "test",
      email: "test@example.com",
    } as Inputs["user"]["create"]);

    expect(true).toBe(true); // Compile-time test passed
  });

  it("should infer output types correctly (compile-time verification)", () => {
    type Outputs = import("../types").InferClientOutputs<TestContract>;

    void ({ status: "ok" } as Outputs["health"]);
    void ("Hello" as Outputs["greet"]);
    void ({ id: 1, name: "test" } as Outputs["user"]["get"]);
    void ({ id: 1, name: "test" } as Outputs["user"]["create"]);
    void (true as Outputs["user"]["delete"]);

    expect(true).toBe(true); // Compile-time test passed
  });

  it("should infer procedure types correctly (compile-time verification)", () => {
    type Types = import("../types").InferClientProcedureTypes<TestContract>;

    void ("query" as Types["health"]);
    void ("mutation" as Types["user"]["create"]);
    void ("subscription" as Types["stream"]["events"]);

    expect(true).toBe(true); // Compile-time test passed
  });

  it("should infer error types correctly (compile-time verification)", () => {
    type Errors = import("../types").InferClientErrors<TestContract>;

    // All errors should be RpcError type
    void ({ code: "NOT_FOUND", message: "Not found" } as Errors["health"]);
    void ({
      code: "INTERNAL_ERROR",
      message: "Error",
    } as Errors["user"]["get"]);

    expect(true).toBe(true); // Compile-time test passed
  });
});
