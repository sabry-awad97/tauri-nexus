/**
 * Unit Type Serialization Tests
 *
 * Tests that the RPC client correctly sends `null` for void/unit inputs
 * instead of `{}`, which would cause Rust deserialization to fail.
 *
 * Background: Rust's serde deserializes `null` as `()` (unit type),
 * but `{}` (empty object) fails with "invalid type: map, expected unit".
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { call } from "../lib/rpc/client";
import { rpc } from "../rpc/contract";

// Mock is set up in setup.ts
const mockInvoke = vi.mocked(invoke);

describe("Unit Type Serialization", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue({});
  });

  describe("call function", () => {
    it("should send null as default input", async () => {
      mockInvoke.mockResolvedValue("result");

      await call("test.procedure");

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "test.procedure",
        input: null,
      });
    });

    it("should send null when explicitly passed", async () => {
      mockInvoke.mockResolvedValue("result");

      await call("test.procedure", null);

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "test.procedure",
        input: null,
      });
    });

    it("should send object when provided", async () => {
      mockInvoke.mockResolvedValue("result");

      await call("test.procedure", { name: "test" });

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "test.procedure",
        input: { name: "test" },
      });
    });

    it("should NOT send empty object for void procedures", async () => {
      mockInvoke.mockResolvedValue("result");

      // This is the bug we fixed - previously it sent {}
      await call("test.procedure");

      const callArgs = mockInvoke.mock.calls[0][1] as { input: unknown };
      expect(callArgs.input).not.toEqual({});
      expect(callArgs.input).toBeNull();
    });
  });

  describe("health procedure", () => {
    it("should send null input for void procedure", async () => {
      mockInvoke.mockResolvedValue({ status: "ok", version: "1.0.0" });

      await rpc.health();

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "health",
        input: null,
      });
    });
  });

  describe("user.list procedure", () => {
    it("should send null input for void procedure", async () => {
      mockInvoke.mockResolvedValue([]);

      await rpc.user.list();

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "user.list",
        input: null,
      });
    });
  });

  describe("user.get procedure", () => {
    it("should send object input for struct procedure", async () => {
      mockInvoke.mockResolvedValue({
        id: 1,
        name: "Test",
        email: "test@test.com",
        createdAt: "",
      });

      await rpc.user.get({ id: 1 });

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "user.get",
        input: { id: 1 },
      });
    });
  });

  describe("user.create procedure", () => {
    it("should send object input for struct procedure", async () => {
      mockInvoke.mockResolvedValue({
        id: 1,
        name: "Test",
        email: "test@test.com",
        createdAt: "",
      });

      await rpc.user.create({ name: "Test", email: "test@test.com" });

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "user.create",
        input: { name: "Test", email: "test@test.com" },
      });
    });
  });
});

describe("JSON Serialization Compatibility", () => {
  it("null should be valid JSON", () => {
    expect(JSON.stringify(null)).toBe("null");
    expect(JSON.parse("null")).toBeNull();
  });

  it("empty object should be different from null", () => {
    expect(JSON.stringify({})).toBe("{}");
    expect(JSON.stringify(null)).not.toBe(JSON.stringify({}));
  });

  it("undefined becomes null in JSON", () => {
    // When undefined is in an object value, it's omitted
    // But as a direct value, JSON.stringify returns undefined (not a string)
    expect(JSON.stringify(undefined)).toBeUndefined();

    // In arrays, undefined becomes null
    expect(JSON.stringify([undefined])).toBe("[null]");
  });
});
