import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { rpc } from "../rpc/contract";
import { useBatch } from "@tauri-nexus/rpc-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";

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
      // Error handled by hook
    }
  };

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

      setTotalDuration(performance.now() - startTime);
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

      setTotalDuration(performance.now() - startTime);
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

      setTotalDuration(performance.now() - startTime);
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
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header>
        <h1 className="text-3xl font-bold mb-2">📦 Batch Requests</h1>
        <p className="text-muted-foreground">
          Execute multiple RPC calls in a single request with full type safety
        </p>
      </header>

      <section className="space-y-4">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <span>⭐</span> useBatch Hook (Recommended)
        </h2>
        <Card className="border-primary/30">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-base">🪝 React Hook</CardTitle>
              <Badge>Recommended</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Automatic state management
            </p>
            <code className="text-xs bg-muted px-2 py-1 rounded block">
              useBatch(() =&gt; rpc.batch()...)
            </code>
            <Button onClick={runHookBatch} disabled={healthBatch.isLoading}>
              {healthBatch.isLoading ? "Running..." : "Run Hook Batch"}
            </Button>

            {healthBatch.isSuccess && (
              <div className="p-4 rounded-lg bg-green-500/10 border border-green-500/30 text-sm space-y-1">
                <p className="text-green-500">
                  ✓ Success in {healthBatch.duration?.toFixed(2)}ms
                </p>
                <p>Health: {healthBatch.getResult("health")?.data?.status}</p>
                <p>Greeting: {healthBatch.getResult("greeting")?.data}</p>
                <p>
                  Users: {healthBatch.getResult("users")?.data?.length} found
                </p>
              </div>
            )}

            {healthBatch.isError && (
              <div className="p-4 rounded-lg bg-destructive/10 border border-destructive/30 text-sm text-destructive">
                ✗ Error: {healthBatch.error?.message}
              </div>
            )}
          </CardContent>
        </Card>
      </section>

      <section className="space-y-4">
        <h2 className="text-lg font-semibold">Direct API Examples</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Basic Batch</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-muted-foreground">
                Health, greeting, and user list
              </p>
              <code className="text-xs bg-muted px-2 py-1 rounded block">
                rpc.batch().add(...).execute()
              </code>
              <Button
                onClick={runBasicBatch}
                disabled={isLoading}
                className="w-full"
              >
                {isLoading ? "Running..." : "Run Basic Batch"}
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">👥 User Operations</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-muted-foreground">
                Fetch multiple users in parallel
              </p>
              <code className="text-xs bg-muted px-2 py-1 rounded block">
                Multiple user.get calls
              </code>
              <Button
                onClick={runUserBatch}
                disabled={isLoading}
                className="w-full"
              >
                {isLoading ? "Running..." : "Run User Batch"}
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">📊 Large Batch</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-muted-foreground">
                8 requests in single call
              </p>
              <code className="text-xs bg-muted px-2 py-1 rounded block">
                Reduced IPC overhead
              </code>
              <Button
                onClick={runLargeBatch}
                disabled={isLoading}
                className="w-full"
              >
                {isLoading ? "Running..." : "Run Large Batch"}
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>

      {results.length > 0 && (
        <section className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">Results</h2>
            <div className="flex gap-3 text-sm">
              <span className="text-green-500">✓ {successCount} succeeded</span>
              <span className="text-destructive">✗ {errorCount} failed</span>
              {totalDuration && (
                <span className="text-muted-foreground">
                  ⏱ {totalDuration.toFixed(2)}ms
                </span>
              )}
            </div>
          </div>

          <ScrollArea className="h-[300px]">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {results.map((result) => (
                <Card
                  key={result.id}
                  className={
                    result.success
                      ? "border-green-500/30"
                      : "border-destructive/30"
                  }
                >
                  <CardHeader className="py-3">
                    <div className="flex items-center justify-between">
                      <span className="font-mono text-sm">{result.id}</span>
                      <Badge
                        variant={result.success ? "default" : "destructive"}
                      >
                        {result.success ? "✓ Success" : "✗ Failed"}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {result.path}
                    </p>
                  </CardHeader>
                  <CardContent className="pt-0">
                    {result.success ? (
                      <pre className="text-xs bg-muted p-2 rounded overflow-auto max-h-24">
                        {JSON.stringify(result.data, null, 2)}
                      </pre>
                    ) : (
                      <div className="text-xs text-destructive">
                        <span className="font-mono">{result.error?.code}</span>:{" "}
                        {result.error?.message}
                      </div>
                    )}
                  </CardContent>
                </Card>
              ))}
            </div>
          </ScrollArea>
        </section>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="text-sm text-muted-foreground">
            Code Example
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="bg-muted rounded-lg p-4 overflow-x-auto">
            <pre className="text-xs font-mono text-muted-foreground">{`import { useBatch } from "../lib/rpc";
import { rpc } from "../rpc/contract";

function MyComponent() {
  const batch = useBatch(
    () => rpc.batch()
      .add("health", "health", undefined)
      .add("user", "user.get", { id: 1 })
      .add("greeting", "greet", { name: "World" }),
    { executeOnMount: true }
  );

  if (batch.isLoading) return <div>Loading...</div>;
  if (batch.isError) return <div>Error: {batch.error?.message}</div>;

  const healthResult = batch.getResult("health");
  const userResult = batch.getResult("user");

  return (
    <div>
      <p>Health: {healthResult?.data?.status}</p>
      <p>User: {userResult?.data?.name}</p>
    </div>
  );
}`}</pre>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export const Route = createFileRoute("/batch")({
  component: BatchExample,
});
