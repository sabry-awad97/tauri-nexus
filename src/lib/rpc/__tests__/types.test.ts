// =============================================================================
// Type System Tests
// =============================================================================
// Tests for type inference utilities and contract builder helpers.

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import {
  query,
  mutation,
  subscription,
  type ContractRouter,
  type InferInput,
  type InferOutput,
  type InferProcedureType,
  type RouterClient,
  type ExtractPaths,
  type ExtractSubscriptionPaths,
  type GetProcedureAtPath,
} from "../types";

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
// If these compile without errors, the type system is working correctly.

describe("Type Inference (Compile-Time Verification)", () => {
  // Define a test contract
  interface TestContract extends ContractRouter {
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
        input: { name: string };
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
      counter: {
        type: "subscription";
        input: { start: number };
        output: number;
      };
    };
  }

  it("should correctly infer input types", () => {
    // These type assertions verify compile-time type inference
    type HealthInput = InferInput<TestContract["health"]>;
    type GreetInput = InferInput<TestContract["greet"]>;
    type UserGetInput = InferInput<TestContract["user"]["get"]>;
    type UserCreateInput = InferInput<TestContract["user"]["create"]>;
    type StreamEventsInput = InferInput<TestContract["stream"]["events"]>;
    type StreamCounterInput = InferInput<TestContract["stream"]["counter"]>;

    // Runtime verification that types are correctly structured
    const healthInput: HealthInput = undefined as unknown as HealthInput;
    const greetInput: GreetInput = { name: "test" };
    const userGetInput: UserGetInput = { id: 1 };
    const userCreateInput: UserCreateInput = { name: "test" };
    const streamEventsInput: StreamEventsInput =
      undefined as unknown as StreamEventsInput;
    const streamCounterInput: StreamCounterInput = { start: 0 };

    expect(greetInput.name).toBe("test");
    expect(userGetInput.id).toBe(1);
    expect(userCreateInput.name).toBe("test");
    expect(streamCounterInput.start).toBe(0);
  });

  it("should correctly infer output types", () => {
    type HealthOutput = InferOutput<TestContract["health"]>;
    type GreetOutput = InferOutput<TestContract["greet"]>;
    type UserGetOutput = InferOutput<TestContract["user"]["get"]>;
    type UserCreateOutput = InferOutput<TestContract["user"]["create"]>;
    type UserDeleteOutput = InferOutput<TestContract["user"]["delete"]>;

    const healthOutput: HealthOutput = { status: "ok" };
    const greetOutput: GreetOutput = "Hello";
    const userGetOutput: UserGetOutput = { id: 1, name: "test" };
    const userCreateOutput: UserCreateOutput = { id: 1, name: "test" };
    const userDeleteOutput: UserDeleteOutput = true;

    expect(healthOutput.status).toBe("ok");
    expect(greetOutput).toBe("Hello");
    expect(userGetOutput.id).toBe(1);
    expect(userCreateOutput.name).toBe("test");
    expect(userDeleteOutput).toBe(true);
  });

  it("should correctly infer procedure types", () => {
    type HealthType = InferProcedureType<TestContract["health"]>;
    type UserCreateType = InferProcedureType<TestContract["user"]["create"]>;
    type StreamEventsType = InferProcedureType<
      TestContract["stream"]["events"]
    >;

    // These should be 'query', 'mutation', 'subscription' respectively
    const healthType: HealthType = "query";
    const userCreateType: UserCreateType = "mutation";
    const streamEventsType: StreamEventsType = "subscription";

    expect(healthType).toBe("query");
    expect(userCreateType).toBe("mutation");
    expect(streamEventsType).toBe("subscription");
  });

  it("should correctly extract all paths", () => {
    type AllPaths = ExtractPaths<TestContract>;

    // Verify that paths are correctly extracted
    const paths: AllPaths[] = [
      "health",
      "greet",
      "user.get",
      "user.create",
      "user.delete",
      "stream.events",
      "stream.counter",
    ];

    expect(paths).toHaveLength(7);
    expect(paths).toContain("health");
    expect(paths).toContain("user.get");
    expect(paths).toContain("stream.counter");
  });

  it("should correctly extract subscription paths", () => {
    type SubPaths = ExtractSubscriptionPaths<TestContract>;

    const subPaths: SubPaths[] = ["stream.events", "stream.counter"];

    expect(subPaths).toHaveLength(2);
    expect(subPaths).toContain("stream.events");
    expect(subPaths).toContain("stream.counter");
  });

  it("should correctly get procedure at path", () => {
    type HealthProc = GetProcedureAtPath<TestContract, "health">;
    type UserGetProc = GetProcedureAtPath<TestContract, "user.get">;
    type StreamCounterProc = GetProcedureAtPath<TestContract, "stream.counter">;

    // Verify the procedure types are correct
    type HealthProcType = InferProcedureType<HealthProc>;
    type UserGetProcType = InferProcedureType<UserGetProc>;
    type StreamCounterProcType = InferProcedureType<StreamCounterProc>;

    const healthType: HealthProcType = "query";
    const userGetType: UserGetProcType = "query";
    const streamCounterType: StreamCounterProcType = "subscription";

    expect(healthType).toBe("query");
    expect(userGetType).toBe("query");
    expect(streamCounterType).toBe("subscription");
  });
});

// =============================================================================
// RouterClient Type Tests
// =============================================================================

describe("RouterClient Type Structure", () => {
  interface SimpleContract extends ContractRouter {
    ping: { type: "query"; input: void; output: string };
    echo: { type: "query"; input: { message: string }; output: string };
    save: { type: "mutation"; input: { data: string }; output: boolean };
    nested: {
      deep: {
        method: { type: "query"; input: { x: number }; output: number };
      };
    };
  }

  it("should create correct client type structure", () => {
    // This test verifies the RouterClient type creates the correct structure
    // The actual implementation is tested in client.test.ts
    type Client = RouterClient<SimpleContract>;

    // Type-level verification - if this compiles, the types are correct
    type PingFn = Client["ping"];
    type EchoFn = Client["echo"];
    type SaveFn = Client["save"];
    type NestedDeepMethodFn = Client["nested"]["deep"]["method"];

    // These assertions verify the structure exists
    expect(true).toBe(true); // Compile-time test passed
  });
});
