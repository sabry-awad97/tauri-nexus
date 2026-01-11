import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { rpc } from "../rpc/contract";
import { useBatch } from "@tauri-nexus/rpc-react";

interface BatchResultDisplay {
  id: string;
  path: string;
  success: boolean;
  data?: unknown;
  error?: { code: string; message: string };
}

function BatchExample() {
  const [results, setResults] = useState<BatchResultDisplay[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [totalDuration, setTotalDuration] = useState<number | null>(null);

  // ==========================================================================
  // Hook-based batch (recommended for React components)
  // ==========================================================================
  const healthBatch = useBatch(
    () =>
      rpc
        .batch()
        .add("health", "health", undefined)
        .add("greeting", "greet", { name: "Hook User" })
        .add("users", "user.list", undefined),
    { executeOnMount: false },
  );

  const runHookBatch = async () => {
    try {
      await healthBatch.execute();
    } catch {
      // Error is handled by the hook
    }
  };

  // ==========================================================================
  // Direct API examples
  // ==========================================================================

  // Example 1: Type-Safe Batch - Basic
  const runBasicBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await rpc
        .batch()
        .add("health-check", "health", undefined)
        .add("greeting", "greet", { name: "TypeSafe" })
        .add("user-list", "user.list", undefined)
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);

      const healthResult = response.getResult("health-check");
      const greetResult = response.getResult("greeting");

      console.log("Health status:", healthResult.data?.status);
      console.log("Greeting:", greetResult.data);

      setResults(
        response.results.map((r, i) => ({
          id: r.id,
          path: ["health", "greet", "user.list"][i] || "unknown",
          success: !r.error,
          data: r.data,
          error: r.error
            ? { code: r.error.code, message: r.error.message }
            : undefined,
        })),
      );
    } catch (error) {
      console.error("Batch failed:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Example 2: Type-Safe Batch with User Operations
  const runUserBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await rpc
        .batch()
        .add("user-1", "user.get", { id: 1 })
        .add("user-2", "user.get", { id: 2 })
        .add("user-3", "user.get", { id: 3 })
        .add("all-users", "user.list", undefined)
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);

      const user1 = response.getResult("user-1");
      const user2 = response.getResult("user-2");

      console.log("User 1:", user1.data?.name);
      console.log("User 2:", user2.data?.name);

      setResults(
        response.results.map((r, i) => ({
          id: r.id,
          path:
            ["user.get", "user.get", "user.get", "user.list"][i] || "unknown",
          success: !r.error,
          data: r.data,
          error: r.error
            ? { code: r.error.code, message: r.error.message }
            : undefined,
        })),
      );
    } catch (error) {
      console.error("Batch failed:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Example 3: Large batch
  const runLargeBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await rpc
        .batch()
        .add("h1", "health", undefined)
        .add("g1", "greet", { name: "Alice" })
        .add("g2", "greet", { name: "Bob" })
        .add("g3", "greet", { name: "Charlie" })
        .add("g4", "greet", { name: "Diana" })
        .add("u1", "user.get", { id: 1 })
        .add("u2", "user.get", { id: 2 })
        .add("ul", "user.list", undefined)
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);

      const successful = response.getSuccessful();
      console.log(`${successful.length} requests succeeded`);

      setResults(
        response.results.map((r) => ({
          id: r.id,
          path: r.id.startsWith("h")
            ? "health"
            : r.id.startsWith("g")
              ? "greet"
              : r.id === "ul"
                ? "user.list"
                : "user.get",
          success: !r.error,
          data: r.data,
          error: r.error
            ? { code: r.error.code, message: r.error.message }
            : undefined,
        })),
      );
    } catch (error) {
      console.error("Batch failed:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const successCount = results.filter((r) => r.success).length;
  const errorCount = results.filter((r) => !r.success).length;

  return (
    <div className="page batch-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">📦 Batch Requests</h1>
          <p className="page-subtitle">
            Execute multiple RPC calls in a single request with full type safety
          </p>
        </div>
      </header>

      <div className="batch-layout">
        {/* Hook-based Example */}
        <section className="batch-examples">
          <h2>⭐ useBatch Hook (Recommended)</h2>

          <div className="example-cards">
            <div className="example-card featured">
              <h3>🪝 React Hook</h3>
              <p>Automatic state management</p>
              <code>useBatch(() =&gt; rpc.batch()...)</code>
              <button onClick={runHookBatch} disabled={healthBatch.isLoading}>
                {healthBatch.isLoading ? "Running..." : "Run Hook Batch"}
              </button>

              {healthBatch.isSuccess && (
                <div className="hook-results">
                  <p>✓ Success in {healthBatch.duration?.toFixed(2)}ms</p>
                  <p>Health: {healthBatch.getResult("health")?.data?.status}</p>
                  <p>Greeting: {healthBatch.getResult("greeting")?.data}</p>
                  <p>
                    Users: {healthBatch.getResult("users")?.data?.length} found
                  </p>
                </div>
              )}

              {healthBatch.isError && (
                <div className="hook-error">
                  <p>✗ Error: {healthBatch.error?.message}</p>
                </div>
              )}
            </div>
          </div>
        </section>

        {/* Direct API Examples */}
        <section className="batch-examples">
          <h2>Direct API Examples</h2>

          <div className="example-cards">
            <div className="example-card">
              <h3>Basic Batch</h3>
              <p>Health, greeting, and user list</p>
              <code>rpc.batch().add(...).execute()</code>
              <button onClick={runBasicBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run Basic Batch"}
              </button>
            </div>

            <div className="example-card">
              <h3>👥 User Operations</h3>
              <p>Fetch multiple users in parallel</p>
              <code>Multiple user.get calls</code>
              <button onClick={runUserBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run User Batch"}
              </button>
            </div>

            <div className="example-card">
              <h3>📊 Large Batch</h3>
              <p>8 requests in single call</p>
              <code>Reduced IPC overhead</code>
              <button onClick={runLargeBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run Large Batch"}
              </button>
            </div>
          </div>
        </section>

        {/* Results Section */}
        {results.length > 0 && (
          <section className="batch-results">
            <div className="results-header">
              <h2>Results</h2>
              <div className="results-stats">
                <span className="stat success">✓ {successCount} succeeded</span>
                <span className="stat error">✗ {errorCount} failed</span>
                {totalDuration && (
                  <span className="stat duration">
                    ⏱ {totalDuration.toFixed(2)}ms total
                  </span>
                )}
              </div>
            </div>

            <div className="results-grid">
              {results.map((result) => (
                <div
                  key={result.id}
                  className={`result-card ${result.success ? "success" : "error"}`}
                >
                  <div className="result-header">
                    <span className="result-id">{result.id}</span>
                    <span
                      className={`result-status ${result.success ? "success" : "error"}`}
                    >
                      {result.success ? "✓ Success" : "✗ Failed"}
                    </span>
                  </div>
                  <div className="result-path">{result.path}</div>
                  {result.success ? (
                    <pre className="result-data">
                      {JSON.stringify(result.data, null, 2)}
                    </pre>
                  ) : (
                    <div className="result-error">
                      <span className="error-code">{result.error?.code}</span>
                      <span className="error-message">
                        {result.error?.message}
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Code Examples */}
        <section className="code-examples">
          <h2>Code Examples</h2>

          <div className="code-block featured">
            <h4>⭐ useBatch Hook (Recommended for React)</h4>
            <pre>{`import { useBatch } from "../lib/rpc";
import { rpc } from "../rpc/contract";

function MyComponent() {
  const batch = useBatch(
    () => rpc.batch()
      .add("health", "health", undefined)
      .add("user", "user.get", { id: 1 })
      .add("greeting", "greet", { name: "World" }),
    { executeOnMount: true }  // or false for manual execution
  );

  if (batch.isLoading) return <div>Loading...</div>;
  if (batch.isError) return <div>Error: {batch.error?.message}</div>;

  // Results are typed!
  const healthResult = batch.getResult("health");
  const userResult = batch.getResult("user");

  return (
    <div>
      <p>Health: {healthResult?.data?.status}</p>
      <p>User: {userResult?.data?.name}</p>
      <p>Duration: {batch.duration}ms</p>
      <button onClick={() => batch.execute()}>Refresh</button>
    </div>
  );
}`}</pre>
          </div>

          <div className="code-block">
            <h4>Direct API (for non-React or manual control)</h4>
            <pre>{`import { rpc } from "../rpc/contract";

// Full type safety! Paths autocomplete, inputs validated at compile time
const response = await rpc.batch()
  .add("health", "health", undefined)           // input: void
  .add("user", "user.get", { id: 1 })           // input: { id: number }
  .add("greeting", "greet", { name: "World" })  // input: { name: string }
  .execute();

// Results are typed per request ID!
const healthResult = response.getResult("health");
if (healthResult.data) {
  console.log(healthResult.data.status);  // HealthResponse
}

const userResult = response.getResult("user");
if (userResult.data) {
  console.log(userResult.data.name);  // User
}`}</pre>
          </div>

          <div className="code-block">
            <h4>Hook Options</h4>
            <pre>{`const batch = useBatch(
  () => rpc.batch().add("h", "health", undefined),
  {
    executeOnMount: true,     // Execute immediately
    onSuccess: (response) => {
      console.log("Batch succeeded:", response.successCount);
    },
    onError: (error) => {
      console.error("Batch failed:", error.message);
    },
  }
);

// Hook state
batch.isLoading;    // boolean
batch.isSuccess;    // boolean
batch.isError;      // boolean
batch.error;        // RpcError | null
batch.duration;     // number | null (ms)
batch.response;     // TypedBatchResponseWrapper | null

// Hook methods
batch.execute();    // Execute the batch
batch.reset();      // Reset state
batch.getResult(id); // Get typed result by ID`}</pre>
          </div>
        </section>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/batch")({
  component: BatchExample,
});
