import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { rpc } from "../rpc/contract";

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

  // Example 1: Type-Safe Batch - Basic
  const runBasicBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      // Full type safety! Paths autocomplete, inputs are validated at compile time
      const response = await rpc
        .batch()
        .add("health-check", "health", undefined) // input: void
        .add("greeting", "greet", { name: "TypeSafe" }) // input: { name: string }
        .add("user-list", "user.list", undefined) // input: void
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);

      // Results are typed per request ID!
      const healthResult = response.getResult("health-check");
      const greetResult = response.getResult("greeting");

      // TypeScript knows the types:
      // healthResult.data is HealthResponse | undefined
      // greetResult.data is string | undefined

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
        .add("user-1", "user.get", { id: 1 }) // input: { id: number }
        .add("user-2", "user.get", { id: 2 }) // input: { id: number }
        .add("user-3", "user.get", { id: 3 }) // input: { id: number }
        .add("all-users", "user.list", undefined) // input: void
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);

      // Get typed results
      const user1 = response.getResult("user-1");
      const user2 = response.getResult("user-2");

      // TypeScript knows user1.data is User | undefined
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

  // Example 3: Batch with mixed success/failure
  const runMixedBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await rpc
        .batch()
        .add("success-1", "health", undefined)
        .add("success-2", "greet", { name: "Still Works" })
        .add("success-3", "user.list", undefined)
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);

      // Check success/failure counts
      console.log(
        `Success: ${response.successCount}, Errors: ${response.errorCount}`,
      );

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

  // Example 4: Large batch with many requests
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

      // Get all successful results
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
        {/* Examples Section */}
        <section className="batch-examples">
          <h2>Type-Safe Batch Examples</h2>

          <div className="example-cards">
            <div className="example-card featured">
              <h3>⭐ Basic Batch</h3>
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
              <h3>✅ Mixed Results</h3>
              <p>Batch with success tracking</p>
              <code>response.successCount</code>
              <button onClick={runMixedBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run Mixed Batch"}
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
            <h4>⭐ Type-Safe Batch (Recommended)</h4>
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
            <h4>Response Helper Methods</h4>
            <pre>{`const response = await rpc.batch()
  .add("h", "health", undefined)
  .add("u", "user.get", { id: 1 })
  .execute();

// Check success/failure
response.isSuccess("h");     // boolean
response.isError("u");       // boolean

// Get counts
response.successCount;       // number
response.errorCount;         // number

// Get filtered results
response.getSuccessful();    // TypedBatchResult[]
response.getFailed();        // TypedBatchResult[]

// Get all results in order
response.results;            // TypedBatchResult[]`}</pre>
          </div>
        </section>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/batch")({
  component: BatchExample,
});
