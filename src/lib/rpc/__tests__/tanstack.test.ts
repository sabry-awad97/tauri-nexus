// =============================================================================
// TanStack Query Integration Tests
// =============================================================================

import { describe, it, expect, vi } from "vitest";
import { createTanstackQueryUtils } from "../tanstack";

// =============================================================================
// Mock Client
// =============================================================================

function createMockClient() {
  return {
    health: vi.fn().mockResolvedValue({ status: "ok", version: "1.0.0" }),
    greet: vi.fn().mockImplementation(({ name }) => Promise.resolve(`Hello, ${name}!`)),
    user: {
      get: vi.fn().mockImplementation(({ id }) => 
        Promise.resolve({ id, name: "Test User", email: "test@example.com" })
      ),
      list: vi.fn().mockResolvedValue([
        { id: 1, name: "User 1", email: "user1@example.com" },
        { id: 2, name: "User 2", email: "user2@example.com" },
      ]),
      create: vi.fn().mockImplementation((input) => 
        Promise.resolve({ id: 1, ...input })
      ),
      delete: vi.fn().mockResolvedValue({ success: true }),
    },
  };
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function createUtils(client: any) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return createTanstackQueryUtils(client) as any;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function createUtilsWithPath(client: any, path: string[]) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return createTanstackQueryUtils(client, { path }) as any;
}

// =============================================================================
// Tests
// =============================================================================

describe("createTanstackQueryUtils", () => {
  describe("query options", () => {
    it("generates query options for procedure without input", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.health.queryOptions();

      expect(options.queryKey).toEqual(["health"]);
      expect(options.queryFn).toBeInstanceOf(Function);
      expect(options.enabled).toBeUndefined();
    });

    it("generates query options for procedure with input", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.greet.queryOptions({ input: { name: "World" } });

      expect(options.queryKey).toEqual(["greet", { name: "World" }]);
      expect(options.queryFn).toBeInstanceOf(Function);
    });

    it("includes enabled option when provided", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.health.queryOptions({ enabled: false });

      expect(options.enabled).toBe(false);
    });

    it("queryFn calls the underlying client function", async () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.greet.queryOptions({ input: { name: "Test" } });
      const result = await options.queryFn();

      expect(client.greet).toHaveBeenCalledWith({ name: "Test" });
      expect(result).toBe("Hello, Test!");
    });

    it("queryFn works for procedures without input", async () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.health.queryOptions();
      const result = await options.queryFn();

      expect(client.health).toHaveBeenCalled();
      expect(result).toEqual({ status: "ok", version: "1.0.0" });
    });
  });

  describe("nested routers", () => {
    it("generates query options for nested procedures", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.user.get.queryOptions({ input: { id: 1 } });

      expect(options.queryKey).toEqual(["user", "get", { id: 1 }]);
    });

    it("generates query options for nested procedures without input", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.user.list.queryOptions();

      expect(options.queryKey).toEqual(["user", "list"]);
    });

    it("queryFn calls nested client function", async () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.user.get.queryOptions({ input: { id: 42 } });
      const result = await options.queryFn();

      expect(client.user.get).toHaveBeenCalledWith({ id: 42 });
      expect(result).toEqual({ id: 42, name: "Test User", email: "test@example.com" });
    });
  });

  describe("mutation options", () => {
    it("generates mutation options", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.user.create.mutationOptions();

      expect(options.mutationKey).toEqual(["user", "create"]);
      expect(options.mutationFn).toBeInstanceOf(Function);
    });

    it("mutationFn calls the underlying client function", async () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const options = utils.user.create.mutationOptions();
      const result = await options.mutationFn({ name: "New User", email: "new@example.com" });

      expect(client.user.create).toHaveBeenCalledWith({ name: "New User", email: "new@example.com" });
      expect(result).toEqual({ id: 1, name: "New User", email: "new@example.com" });
    });
  });

  describe("query keys", () => {
    it("generates queryKey for procedure without input", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.health.queryKey();

      expect(key).toEqual(["health"]);
    });

    it("generates queryKey for procedure with input", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.greet.queryKey({ input: { name: "World" } });

      expect(key).toEqual(["greet", { name: "World" }]);
    });

    it("generates mutationKey", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.user.create.mutationKey();

      expect(key).toEqual(["user", "create"]);
    });
  });

  describe("partial keys for invalidation", () => {
    it("generates partial key for root", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.key();

      expect(key).toEqual([]);
    });

    it("generates partial key for namespace", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.user.key();

      expect(key).toEqual(["user"]);
    });

    it("generates partial key for procedure", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.user.get.key();

      expect(key).toEqual(["user", "get"]);
    });

    it("generates key with input for specific cache entry", () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const key = utils.user.get.key({ input: { id: 1 } });

      expect(key).toEqual(["user", "get", { id: 1 }]);
    });
  });

  describe("direct call", () => {
    it("calls procedure directly", async () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const result = await utils.greet.call({ name: "Direct" });

      expect(client.greet).toHaveBeenCalledWith({ name: "Direct" });
      expect(result).toBe("Hello, Direct!");
    });

    it("calls nested procedure directly", async () => {
      const client = createMockClient();
      const utils = createUtils(client);

      const result = await utils.user.list.call();

      expect(client.user.list).toHaveBeenCalled();
      expect(result).toHaveLength(2);
    });
  });

  describe("base path option", () => {
    it("prepends base path to all keys", () => {
      const client = createMockClient();
      const utils = createUtilsWithPath(client, ["api"]);

      expect(utils.key()).toEqual(["api"]);
      expect(utils.health.queryKey()).toEqual(["api", "health"]);
      expect(utils.user.key()).toEqual(["api", "user"]);
      expect(utils.user.get.queryKey({ input: { id: 1 } })).toEqual(["api", "user", "get", { id: 1 }]);
    });

    it("works with nested base paths", () => {
      const client = createMockClient();
      const utils = createUtilsWithPath(client, ["v1", "api"]);

      expect(utils.health.queryKey()).toEqual(["v1", "api", "health"]);
    });
  });
});
