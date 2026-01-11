import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { callBatch, createBatch, type BatchResponse, type SingleRequest } from "../lib/rpc";

interface BatchResultDisplay {
  id: string;
  path: string;
  success: boolean;
  data?: unknown;
  error?: { code: string; message: string };
  duration?: number;
}

function BatchExample() {
  const [results, setResults] = useState<BatchResultDisplay[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [totalDuration, setTotalDuration] = useState<number | null>(null);
  const [customRequests, setCustomRequests] = useState<SingleRequest[]>([
    { id: "1", path: "health", input: null },
    { id: "2", path: "greet", input: { name: "Alice" } },
    { id: "3", path: "greet", input: { name: "Bob" } },
  ]);
  const [newPath, setNewPath] = useState("health");
  const [newInput, setNewInput] = useState("");

  // Example 1: Basic batch with callBatch function
  const runBasicBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await callBatch([
        { id: "health-1", path: "health", input: null },
        { id: "greet-alice", path: "greet", input: { name: "Alice" } },
        { id: "greet-bob", path: "greet", input: { name: "Bob" } },
        { id: "user-list", path: "user.list", input: null },
      ]);

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);
      setResults(formatResults(response));
    } catch (error) {
      console.error("Batch failed:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Example 2: Using BatchBuilder fluent API
  const runBuilderBatch = async () => {
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await createBatch()
        .add("req-1", "health", null)
        .add("req-2", "greet", { name: "Builder Test" })
        .add("req-3", "user.list", null)
        .add("req-4", "greet", { name: "Fluent API" })
        .execute();

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);
      setResults(formatResults(response));
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
      const response = await callBatch([
        { id: "success-1", path: "health", input: null },
        { id: "fail-1", path: "nonexistent.procedure", input: null },
        { id: "success-2", path: "greet", input: { name: "Still Works" } },
        { id: "fail-2", path: "another.missing", input: null },
        { id: "success-3", path: "user.list", input: null },
      ]);

      const endTime = performance.now();
      setTotalDuration(endTime - startTime);
      setResults(formatResults(response));
    } catch (error) {
      console.error("Batch failed:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Example 4: Custom batch
  const runCustomBatch = async () => {
    if (customRequests.length === 0) return;
    
    setIsLoading(true);
    setResults([]);
    const startTime = performance.now();

    try {
      const response = await callBatch(customRequests);
      const endTime = performance.now();
      setTotalDuration(endTime - startTime);
      setResults(formatResults(response));
    } catch (error) {
      console.error("Batch failed:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const formatResults = (response: BatchResponse): BatchResultDisplay[] => {
    return response.results.map((result) => ({
      id: result.id,
      path: customRequests.find(r => r.id === result.id)?.path || "unknown",
      success: !result.error,
      data: result.data,
      error: result.error ? { code: result.error.code, message: result.error.message } : undefined,
    }));
  };

  const addCustomRequest = () => {
    let parsedInput: unknown = null;
    if (newInput.trim()) {
      try {
        parsedInput = JSON.parse(newInput);
      } catch {
        parsedInput = newInput;
      }
    }

    const newRequest: SingleRequest = {
      id: `custom-${Date.now()}`,
      path: newPath,
      input: parsedInput,
    };

    setCustomRequests([...customRequests, newRequest]);
    setNewInput("");
  };

  const removeCustomRequest = (id: string) => {
    setCustomRequests(customRequests.filter(r => r.id !== id));
  };

  const successCount = results.filter(r => r.success).length;
  const errorCount = results.filter(r => !r.success).length;

  return (
    <div className="page batch-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">📦 Batch Requests</h1>
          <p className="page-subtitle">
            Execute multiple RPC calls in a single request for reduced IPC overhead
          </p>
        </div>
      </header>

      <div className="batch-layout">
        {/* Examples Section */}
        <section className="batch-examples">
          <h2>Quick Examples</h2>
          
          <div className="example-cards">
            <div className="example-card">
              <h3>Basic Batch</h3>
              <p>Execute 4 different procedures in parallel</p>
              <code>callBatch([...])</code>
              <button onClick={runBasicBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run Basic Batch"}
              </button>
            </div>

            <div className="example-card">
              <h3>Builder Pattern</h3>
              <p>Use fluent API to build batch requests</p>
              <code>createBatch().add(...).execute()</code>
              <button onClick={runBuilderBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run Builder Batch"}
              </button>
            </div>

            <div className="example-card">
              <h3>Error Isolation</h3>
              <p>Mix of successful and failing requests</p>
              <code>Individual failures don't affect others</code>
              <button onClick={runMixedBatch} disabled={isLoading}>
                {isLoading ? "Running..." : "Run Mixed Batch"}
              </button>
            </div>
          </div>
        </section>

        {/* Custom Batch Builder */}
        <section className="custom-batch">
          <h2>Custom Batch Builder</h2>
          
          <div className="custom-form">
            <div className="form-row">
              <input
                type="text"
                placeholder="Procedure path (e.g., health, greet, user.list)"
                value={newPath}
                onChange={(e) => setNewPath(e.target.value)}
                className="form-input"
              />
              <input
                type="text"
                placeholder='Input JSON (e.g., {"name": "Test"})'
                value={newInput}
                onChange={(e) => setNewInput(e.target.value)}
                className="form-input"
              />
              <button onClick={addCustomRequest} className="add-btn">
                Add Request
              </button>
            </div>

            {customRequests.length > 0 && (
              <div className="request-list">
                <h4>Pending Requests ({customRequests.length})</h4>
                {customRequests.map((req) => (
                  <div key={req.id} className="request-item">
                    <span className="request-path">{req.path}</span>
                    <span className="request-input">
                      {JSON.stringify(req.input)}
                    </span>
                    <button
                      onClick={() => removeCustomRequest(req.id)}
                      className="remove-btn"
                    >
                      ✕
                    </button>
                  </div>
                ))}
                <button
                  onClick={runCustomBatch}
                  disabled={isLoading || customRequests.length === 0}
                  className="execute-btn"
                >
                  {isLoading ? "Executing..." : `Execute Batch (${customRequests.length} requests)`}
                </button>
              </div>
            )}
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
                  <span className="stat duration">⏱ {totalDuration.toFixed(2)}ms total</span>
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
                    <span className={`result-status ${result.success ? "success" : "error"}`}>
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
                      <span className="error-message">{result.error?.message}</span>
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
          
          <div className="code-block">
            <h4>Using callBatch()</h4>
            <pre>{`import { callBatch } from "../lib/rpc";

const response = await callBatch([
  { id: "1", path: "health", input: null },
  { id: "2", path: "greet", input: { name: "Alice" } },
  { id: "3", path: "user.list", input: null },
]);

for (const result of response.results) {
  if (result.error) {
    console.error(\`\${result.id} failed:\`, result.error);
  } else {
    console.log(\`\${result.id} succeeded:\`, result.data);
  }
}`}</pre>
          </div>

          <div className="code-block">
            <h4>Using BatchBuilder</h4>
            <pre>{`import { createBatch } from "../lib/rpc";

const response = await createBatch()
  .add("health-check", "health", null)
  .add("greeting", "greet", { name: "World" })
  .add("users", "user.list", null)
  .execute();

console.log(\`\${response.results.length} results\`);`}</pre>
          </div>
        </section>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/batch")({
  component: BatchExample,
});
