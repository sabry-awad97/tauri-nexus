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
} from "@tauri-nexus/rpc-react";
import type { AppContract } from "../rpc/contract";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";

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
    setLogs((prev) => [
      ...prev.slice(-9),
      `[${new Date().toLocaleTimeString()}] ${msg}`,
    ]);
  };

  const runDemo = async () => {
    setIsLoading(true);
    setLogs([]);
    setResult("");

    const link = new TauriLink<ClientContext>({
      interceptors: [
        async (ctx, next) => {
          addLog(`→ Request: ${ctx.path}`);
          addLog(`  Context: requestId=${ctx.context.requestId}`);
          const res = await next();
          const duration = Date.now() - ctx.context.startTime;
          addLog(`← Response: ${ctx.path} (${duration}ms)`);
          return res;
        },
        async (ctx, next) => {
          ctx.meta.customHeader = "demo-value";
          addLog(`  Added meta: customHeader`);
          return next();
        },
      ],
      onRequest: (ctx) => addLog(`[Hook] onRequest: ${ctx.path}`),
      onResponse: (_data, ctx) => addLog(`[Hook] onResponse: ${ctx.path}`),
      onError: (error, ctx) =>
        addLog(`[Hook] onError: ${ctx.path} - ${error.code}`),
    });

    const client = createClientFromLink<AppContract, ClientContext>(link);

    try {
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
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">
            TauriLink with Interceptors
          </CardTitle>
          <Badge>New</Badge>
        </div>
        <CardDescription>
          Create a client with custom interceptors, client context, and
          lifecycle hooks.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button onClick={runDemo} disabled={isLoading}>
          {isLoading ? (
            <>
              <Spinner className="size-4 mr-2" /> Running...
            </>
          ) : (
            "Run Demo"
          )}
        </Button>

        <div className="bg-muted rounded-lg p-4">
          <h4 className="text-xs font-semibold mb-2 text-muted-foreground">
            Interceptor Logs
          </h4>
          <ScrollArea className="h-32">
            {logs.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Click "Run Demo" to see interceptor logs
              </p>
            ) : (
              <div className="space-y-1">
                {logs.map((log, i) => (
                  <p key={i} className="text-xs font-mono">
                    {log}
                  </p>
                ))}
              </div>
            )}
          </ScrollArea>
        </div>

        {result && (
          <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-4">
            <pre className="text-xs font-mono text-green-500">{result}</pre>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ErrorHandlingDemo() {
  const [testResult, setTestResult] = useState<string>("");

  const runErrorTests = () => {
    const results: string[] = [];

    const validError: RpcError = {
      code: "NOT_FOUND",
      message: "User not found",
    };
    const invalidError = { foo: "bar" };

    results.push(`isRpcError(validError): ${isRpcError(validError)}`);
    results.push(`isRpcError(invalidError): ${isRpcError(invalidError)}`);
    results.push(`isRpcError("string"): ${isRpcError("string")}`);
    results.push(
      `hasErrorCode(validError, "NOT_FOUND"): ${hasErrorCode(validError, "NOT_FOUND")}`,
    );
    results.push(
      `hasErrorCode(validError, "UNAUTHORIZED"): ${hasErrorCode(validError, "UNAUTHORIZED")}`,
    );

    const detailedError: RpcError = {
      code: "VALIDATION_ERROR",
      message: "Invalid input",
      details: { field: "email", reason: "invalid format" },
    };
    results.push(`\nDetailed error: ${JSON.stringify(detailedError, null, 2)}`);

    setTestResult(results.join("\n"));
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">Error Handling Utilities</CardTitle>
          <Badge variant="outline">Utils</Badge>
        </div>
        <CardDescription>
          Type-safe error checking with{" "}
          <code className="bg-muted px-1 rounded text-xs">isRpcError</code> and{" "}
          <code className="bg-muted px-1 rounded text-xs">hasErrorCode</code>.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button onClick={runErrorTests}>Run Error Tests</Button>

        {testResult && (
          <div className="bg-muted rounded-lg p-4">
            <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap">
              {testResult}
            </pre>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">Backend Introspection</CardTitle>
          <Badge variant="outline">Meta</Badge>
        </div>
        <CardDescription>
          Query available procedures and active subscriptions from the backend.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button variant="secondary" onClick={fetchInfo} disabled={isLoading}>
          {isLoading ? (
            <>
              <Spinner className="size-4 mr-2" /> Loading...
            </>
          ) : (
            "Refresh"
          )}
        </Button>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div className="grid grid-cols-2 gap-4">
          <div className="bg-muted rounded-lg p-4 text-center">
            <p className="text-xs text-muted-foreground mb-1">
              Active Subscriptions
            </p>
            <p className="text-2xl font-bold">{subCount ?? "—"}</p>
          </div>
          <div className="bg-muted rounded-lg p-4 text-center">
            <p className="text-xs text-muted-foreground mb-1">
              Total Procedures
            </p>
            <p className="text-2xl font-bold">{procedures.length || "—"}</p>
          </div>
        </div>

        {procedures.length > 0 && (
          <div>
            <h4 className="text-xs font-semibold mb-2 text-muted-foreground">
              Available Procedures
            </h4>
            <div className="flex flex-wrap gap-1.5">
              {procedures.map((proc) => (
                <Badge
                  key={proc}
                  variant="secondary"
                  className="text-xs font-mono"
                >
                  {proc}
                </Badge>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function MiddlewareDemo() {
  const [logs, setLogs] = useState<string[]>([]);

  const setupMiddleware = () => {
    setLogs([]);
    const addLog = (msg: string) => {
      setLogs((prev) => [
        ...prev,
        `[${new Date().toLocaleTimeString()}] ${msg}`,
      ]);
    };

    configureRpc({
      middleware: [
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
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">Global Middleware</CardTitle>
          <Badge variant="outline">Config</Badge>
        </div>
        <CardDescription>
          Configure global middleware for logging, timing, auth, and more.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button onClick={setupMiddleware}>Setup Middleware</Button>

        <div className="bg-muted rounded-lg p-4">
          <h4 className="text-xs font-semibold mb-2 text-muted-foreground">
            Middleware Logs
          </h4>
          <ScrollArea className="h-32">
            {logs.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Click "Setup Middleware" to configure
              </p>
            ) : (
              <div className="space-y-1">
                {logs.map((log, i) => (
                  <p key={i} className="text-xs font-mono">
                    {log}
                  </p>
                ))}
              </div>
            )}
          </ScrollArea>
        </div>
      </CardContent>
    </Card>
  );
}

function InterceptorHelpersDemo() {
  const [output, setOutput] = useState<string>("");

  const showHelpers = () => {
    setOutput(`// Built-in interceptor helpers:

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
});`);
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">Interceptor Helpers</CardTitle>
          <Badge variant="outline">Helpers</Badge>
        </div>
        <CardDescription>
          Pre-built interceptors for common patterns: logging, retry, error
          handling.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button onClick={showHelpers}>Show Examples</Button>

        {output && (
          <div className="bg-muted rounded-lg p-4">
            <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap">
              {output}
            </pre>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function AdvancedPage() {
  return (
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header>
        <h1 className="text-3xl font-bold mb-2">🔧 Advanced Features</h1>
        <p className="text-muted-foreground">
          TauriLink, interceptors, error handling, and backend introspection
        </p>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
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
