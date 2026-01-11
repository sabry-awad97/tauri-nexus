import { createFileRoute } from "@tanstack/react-router";
import { useState, useEffect } from "react";
import {
  TauriLink,
  createClientFromLink,
  isRpcError,
  hasErrorCode,
  getProcedures,
  getSubscriptionCount,
  configureRpc,
  type RpcError,
  type LinkCallOptions,
} from "../lib/rpc";
import type { AppContract } from "../rpc/contract";

// =============================================================================
// TauriLink Demo - Client Context & Interceptors
// =============================================================================

interface ClientContext {
  requestId: string;
  userId?: string;
  startTime: number;
}

function TauriLinkDemo() {
  const [logs, setLogs] = useState<string[]>([]);
  const [result, setResult] = useState<string>("");
  const [isLoading, setIsLoading] = useState(false);

  const addLog = (msg: string) => {
    setLogs((prev) => [...prev.slice(-9), `[${new Date().toLocaleTimeString()}] ${msg}`]);
  };

  const runDemo = async () => {
    setIsLoading(true);
    setLogs([]);
    setResult("");

    // Create a link with interceptors
    const link = new TauriLink<ClientContext>({
      interceptors: [
        // Logging interceptor
        async (ctx, next) => {
          addLog(`→ Request: ${ctx.path}`);
          addLog(`  Context: requestId=${ctx.context.requestId}`);
          const res = await next();
          const duration = Date.now() - ctx.context.startTime;
          addLog(`← Response: ${ctx.path} (${duration}ms)`);
          return res;
        },
        // Custom header interceptor
        async (ctx, next) => {
          ctx.meta.customHeader = "demo-value";
          addLog(`  Added meta: customHeader`);
          return next();
        },
      ],
      onRequest: (ctx) => {
        addLog(`[Hook] onRequest: ${ctx.path}`);
      },
      onResponse: (_data, ctx) => {
        addLog(`[Hook] onResponse: ${ctx.path}`);
      },
      onError: (error, ctx) => {
        addLog(`[Hook] onError: ${ctx.path} - ${error.code}`);
      },
    });

    const client = createClientFromLink<AppContract, ClientContext>(link);

    try {
      // Call with context - health has void input, so options is the first arg
      const options: LinkCallOptions<ClientContext> = {
        context: {
          requestId: `req-${Date.now()}`,
          userId: "demo-user",
          startTime: Date.now(),
        },
      };
      const health = await client.health(options);
      setResult(JSON.stringify(health, null, 2));
    } catch (err) {
      if (isRpcError(err)) {
        setResult(`Error: ${err.code} - ${err.message}`);
      } else {
        setResult(`Unknown error: ${err}`);
      }
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>TauriLink with Interceptors</h3>
        <span className="demo-badge">New</span>
      </div>
      <p className="demo-description">
        Create a client with custom interceptors, client context, and lifecycle hooks.
      </p>

      <button onClick={runDemo} disabled={isLoading} className="demo-btn primary">
        {isLoading ? "Running..." : "Run Demo"}
      </button>

      <div className="logs-panel">
        <h4>Interceptor Logs</h4>
        <div className="logs-list">
          {logs.length === 0 ? (
            <span className="no-logs">Click "Run Demo" to see interceptor logs</span>
          ) : (
            logs.map((log, i) => (
              <div key={i} className="log-item">{log}</div>
            ))
          )}
        </div>
      </div>

      {result && (
        <div className="demo-result success">
          <pre>{result}</pre>
        </div>
      )}

      <div className="code-preview">
        <pre>{`const link = new TauriLink<ClientContext>({
  interceptors: [
    logging(),
    retry({ maxRetries: 3 }),
    async (ctx, next) => {
      ctx.meta.auth = \`Bearer \${ctx.context.token}\`;
      return next();
    },
  ],
});

const client = createClientFromLink<AppContract, ClientContext>(link);
const result = await client.health(undefined, {
  context: { requestId: 'req-123', userId: 'user-1' },
});`}</pre>
      </div>
    </div>
  );
}

// =============================================================================
// Error Handling Demo
// =============================================================================

function ErrorHandlingDemo() {
  const [testResult, setTestResult] = useState<string>("");

  const runErrorTests = () => {
    const results: string[] = [];

    // Test isRpcError
    const validError: RpcError = { code: "NOT_FOUND", message: "User not found" };
    const invalidError = { foo: "bar" };

    results.push(`isRpcError(validError): ${isRpcError(validError)}`);
    results.push(`isRpcError(invalidError): ${isRpcError(invalidError)}`);
    results.push(`isRpcError("string"): ${isRpcError("string")}`);

    // Test hasErrorCode
    results.push(`hasErrorCode(validError, "NOT_FOUND"): ${hasErrorCode(validError, "NOT_FOUND")}`);
    results.push(`hasErrorCode(validError, "UNAUTHORIZED"): ${hasErrorCode(validError, "UNAUTHORIZED")}`);

    // Error with details
    const detailedError: RpcError = {
      code: "VALIDATION_ERROR",
      message: "Invalid input",
      details: { field: "email", reason: "invalid format" },
    };
    results.push(`\nDetailed error: ${JSON.stringify(detailedError, null, 2)}`);

    setTestResult(results.join("\n"));
  };

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>Error Handling Utilities</h3>
        <span className="demo-badge">Utils</span>
      </div>
      <p className="demo-description">
        Type-safe error checking with <code>isRpcError</code> and <code>hasErrorCode</code>.
      </p>

      <button onClick={runErrorTests} className="demo-btn primary">
        Run Error Tests
      </button>

      {testResult && (
        <div className="demo-result">
          <pre>{testResult}</pre>
        </div>
      )}

      <div className="code-preview">
        <pre>{`import { isRpcError, hasErrorCode } from '../lib/rpc';

try {
  await rpc.user.get({ id: 999 });
} catch (error) {
  if (isRpcError(error)) {
    if (hasErrorCode(error, 'NOT_FOUND')) {
      console.log('User not found');
    } else if (hasErrorCode(error, 'UNAUTHORIZED')) {
      console.log('Please login');
    }
  }
}`}</pre>
      </div>
    </div>
  );
}

// =============================================================================
// Backend Info Demo
// =============================================================================

function BackendInfoDemo() {
  const [procedures, setProcedures] = useState<string[]>([]);
  const [subCount, setSubCount] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>("");

  const fetchInfo = async () => {
    setIsLoading(true);
    setError("");
    try {
      const [procs, count] = await Promise.all([
        getProcedures(),
        getSubscriptionCount(),
      ]);
      setProcedures(procs);
      setSubCount(count);
    } catch (err) {
      setError(isRpcError(err) ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchInfo();
  }, []);

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>Backend Introspection</h3>
        <span className="demo-badge">Meta</span>
      </div>
      <p className="demo-description">
        Query available procedures and active subscriptions from the backend.
      </p>

      <button onClick={fetchInfo} disabled={isLoading} className="demo-btn">
        {isLoading ? "Loading..." : "Refresh"}
      </button>

      {error && <div className="demo-result error">{error}</div>}

      <div className="info-grid-small">
        <div className="info-item">
          <span className="info-label">Active Subscriptions</span>
          <span className="info-value">{subCount ?? "—"}</span>
        </div>
        <div className="info-item">
          <span className="info-label">Total Procedures</span>
          <span className="info-value">{procedures.length || "—"}</span>
        </div>
      </div>

      {procedures.length > 0 && (
        <div className="procedures-list">
          <h4>Available Procedures</h4>
          <div className="procedures-grid">
            {procedures.map((proc) => (
              <span key={proc} className="procedure-tag">{proc}</span>
            ))}
          </div>
        </div>
      )}

      <div className="code-preview">
        <pre>{`import { getProcedures, getSubscriptionCount } from '../lib/rpc';

const procedures = await getProcedures();
// ${JSON.stringify(procedures.slice(0, 3))}...

const activeSubscriptions = await getSubscriptionCount();
// ${subCount ?? 0}`}</pre>
      </div>
    </div>
  );
}

// =============================================================================
// Middleware Configuration Demo
// =============================================================================

function MiddlewareDemo() {
  const [logs, setLogs] = useState<string[]>([]);

  const setupMiddleware = () => {
    setLogs([]);
    const addLog = (msg: string) => {
      setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
    };

    configureRpc({
      middleware: [
        // Timing middleware
        async (ctx, next) => {
          const start = Date.now();
          addLog(`[Timing] Start: ${ctx.path}`);
          try {
            const result = await next();
            addLog(`[Timing] End: ${ctx.path} (${Date.now() - start}ms)`);
            return result;
          } catch (error) {
            addLog(`[Timing] Error: ${ctx.path} (${Date.now() - start}ms)`);
            throw error;
          }
        },
        // Logging middleware
        async (ctx, next) => {
          addLog(`[Log] Input: ${JSON.stringify(ctx.input)}`);
          const result = await next();
          addLog(`[Log] Output received`);
          return result;
        },
      ],
      onRequest: (ctx) => addLog(`[Hook] onRequest: ${ctx.path}`),
      onResponse: () => addLog(`[Hook] onResponse`),
      onError: (_ctx, error) => addLog(`[Hook] onError: ${error.code}`),
    });

    addLog("Middleware configured! Make an RPC call to see it in action.");
  };

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>Global Middleware</h3>
        <span className="demo-badge">Config</span>
      </div>
      <p className="demo-description">
        Configure global middleware for logging, timing, auth, and more.
      </p>

      <button onClick={setupMiddleware} className="demo-btn primary">
        Setup Middleware
      </button>

      <div className="logs-panel">
        <h4>Middleware Logs</h4>
        <div className="logs-list">
          {logs.length === 0 ? (
            <span className="no-logs">Click "Setup Middleware" to configure</span>
          ) : (
            logs.map((log, i) => (
              <div key={i} className="log-item">{log}</div>
            ))
          )}
        </div>
      </div>

      <div className="code-preview">
        <pre>{`import { configureRpc } from '../lib/rpc';

configureRpc({
  middleware: [
    async (ctx, next) => {
      console.log(\`Request: \${ctx.path}\`);
      const result = await next();
      console.log(\`Response received\`);
      return result;
    },
  ],
  onError: (ctx, error) => {
    console.error(\`Error in \${ctx.path}: \${error.message}\`);
  },
});`}</pre>
      </div>
    </div>
  );
}

// =============================================================================
// Interceptor Helpers Demo
// =============================================================================

function InterceptorHelpersDemo() {
  const [output, setOutput] = useState<string>("");

  const showHelpers = () => {
    const examples = `// Built-in interceptor helpers:

// 1. logging() - Log all requests/responses
const link = new TauriLink({
  interceptors: [logging({ prefix: '[API]' })],
});

// 2. retry() - Automatic retry on failure
const link = new TauriLink({
  interceptors: [
    retry({
      maxRetries: 3,
      delay: 1000,
      shouldRetry: (error) => error.code === 'SERVICE_UNAVAILABLE',
    }),
  ],
});

// 3. onError() - Error handling interceptor
const link = new TauriLink({
  interceptors: [
    onError((error, ctx) => {
      analytics.track('rpc_error', {
        path: ctx.path,
        code: error.code,
      });
    }),
  ],
});

// Combine multiple interceptors:
const link = new TauriLink({
  interceptors: [
    logging(),
    retry({ maxRetries: 2 }),
    onError(reportToSentry),
    authInterceptor,
  ],
});`;

    setOutput(examples);
  };

  return (
    <div className="demo-card">
      <div className="demo-header">
        <h3>Interceptor Helpers</h3>
        <span className="demo-badge">Helpers</span>
      </div>
      <p className="demo-description">
        Pre-built interceptors for common patterns: logging, retry, error handling.
      </p>

      <button onClick={showHelpers} className="demo-btn primary">
        Show Examples
      </button>

      {output && (
        <div className="demo-result">
          <pre>{output}</pre>
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Main Page
// =============================================================================

function AdvancedPage() {
  return (
    <div className="page advanced-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">🔧 Advanced Features</h1>
          <p className="page-subtitle">
            TauriLink, interceptors, error handling, and backend introspection
          </p>
        </div>
      </header>

      <div className="demos-grid two-col">
        <TauriLinkDemo />
        <ErrorHandlingDemo />
        <BackendInfoDemo />
        <MiddlewareDemo />
        <InterceptorHelpersDemo />
      </div>
    </div>
  );
}

export const Route = createFileRoute("/advanced")({
  component: AdvancedPage,
});
