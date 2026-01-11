import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { orpc, rpc } from "../rpc/contract";

function DirectCallDemo() {
  const [name, setName] = useState("");
  const [greeting, setGreeting] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handleGreet() {
    if (!name.trim()) return;
    setLoading(true);
    setError("");
    try {
      const result = await rpc.greet({ name });
      setGreeting(result);
    } catch (err: any) {
      setError(err.message || "Failed to greet");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>Direct RPC Call</h3>
        <span className="demo-badge">Imperative</span>
      </div>
      <p className="demo-description">
        Call the RPC procedure directly using <code>rpc.greet()</code>
      </p>

      <div className="demo-input-group">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleGreet()}
          placeholder="Enter your name..."
          className="demo-input"
        />
        <button
          onClick={handleGreet}
          disabled={loading || !name.trim()}
          className="demo-btn primary"
        >
          {loading ? (
            <span className="btn-loading">
              <span className="spinner" />
              Greeting...
            </span>
          ) : (
            "Say Hello"
          )}
        </button>
      </div>

      {greeting && (
        <div className="demo-result success">
          <span className="result-icon">✨</span>
          {greeting}
        </div>
      )}
      {error && (
        <div className="demo-result error">
          <span className="result-icon">⚠️</span>
          {error}
        </div>
      )}

      <div className="code-preview">
        <pre>{`const result = await rpc.greet({ name: "${name || "World"}" });
// Returns: "${greeting || "Hello, World! 👋"}"`}</pre>
      </div>
    </div>
  );
}

function HookDemo() {
  const [name, setName] = useState("World");
  const { data, isLoading, error, refetch } = useQuery({
    ...orpc.greet.queryOptions({ input: { name } }),
    enabled: name.length > 0,
  });

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>useGreet Hook</h3>
        <span className="demo-badge reactive">Reactive</span>
      </div>
      <p className="demo-description">
        Reactive query that automatically refetches when input changes
      </p>

      <div className="demo-input-group">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Type to greet..."
          className="demo-input"
        />
        <button
          onClick={() => refetch()}
          disabled={isLoading}
          className="demo-btn"
        >
          Refetch
        </button>
      </div>

      <div
        className={`demo-result ${isLoading ? "loading" : error ? "error" : "success"}`}
      >
        {isLoading && (
          <>
            <span className="spinner" />
            Loading...
          </>
        )}
        {error && (
          <>
            <span className="result-icon">⚠️</span>
            {error.message}
          </>
        )}
        {data && !isLoading && (
          <>
            <span className="result-icon">✨</span>
            {data}
          </>
        )}
      </div>

      <div className="code-preview">
        <pre>{`const { data, isLoading, error, refetch } = useGreet(
  { name: "${name}" },
  { enabled: ${name.length > 0} }
);`}</pre>
      </div>
    </div>
  );
}

function ValidationDemo() {
  const [name, setName] = useState("");
  const { data, isLoading, error } = useQuery({
    ...orpc.greet.queryOptions({ input: { name } }),
    enabled: name.length > 0,
  });

  const validationHints = [
    { test: name.length === 0, message: "Name is required", type: "info" },
    {
      test: name.length > 0 && name.length < 2,
      message: "Name too short",
      type: "warning",
    },
    { test: name.length >= 2, message: "Valid name", type: "success" },
  ];

  const currentHint = validationHints.find((h) => h.test);

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>Input Validation</h3>
        <span className="demo-badge">Backend</span>
      </div>
      <p className="demo-description">
        The backend validates that name is not empty
      </p>

      <div className="demo-input-group">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Try empty or short names..."
          className={`demo-input ${currentHint?.type}`}
        />
      </div>

      {currentHint && (
        <div className={`validation-hint ${currentHint.type}`}>
          {currentHint.message}
        </div>
      )}

      <div
        className={`demo-result ${isLoading ? "loading" : error ? "error" : data ? "success" : "idle"}`}
      >
        {isLoading && (
          <>
            <span className="spinner" /> Validating...
          </>
        )}
        {error && (
          <>
            <span className="result-icon">❌</span> {error.message}
          </>
        )}
        {data && !isLoading && (
          <>
            <span className="result-icon">✅</span> {data}
          </>
        )}
        {!data && !isLoading && !error && (
          <span className="idle-text">Enter a name to test</span>
        )}
      </div>
    </div>
  );
}

function GreetPage() {
  return (
    <div className="page greet-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">👋 Greet</h1>
          <p className="page-subtitle">
            Simple query example demonstrating RPC calls and React hooks
          </p>
        </div>
      </header>

      <div className="demos-grid">
        <DirectCallDemo />
        <HookDemo />
        <ValidationDemo />
      </div>
    </div>
  );
}

export const Route = createFileRoute("/greet")({
  component: GreetPage,
});
