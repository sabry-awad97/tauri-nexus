import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { orpc, rpc } from "../rpc/contract";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import { Alert, AlertDescription } from "@/components/ui/alert";

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
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">Direct RPC Call</CardTitle>
          <Badge variant="outline">Imperative</Badge>
        </div>
        <CardDescription>
          Call the RPC procedure directly using{" "}
          <code className="bg-muted px-1.5 py-0.5 rounded text-xs">
            rpc.greet()
          </code>
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGreet()}
            placeholder="Enter your name..."
            className="flex-1"
          />
          <Button onClick={handleGreet} disabled={loading || !name.trim()}>
            {loading ? (
              <>
                <Spinner className="size-4 mr-2" /> Greeting...
              </>
            ) : (
              "Say Hello"
            )}
          </Button>
        </div>

        {greeting && (
          <Alert className="border-green-500/30 bg-green-500/10 text-green-500">
            <AlertDescription className="flex items-center gap-2">
              <span>✨</span> {greeting}
            </AlertDescription>
          </Alert>
        )}
        {error && (
          <Alert variant="destructive">
            <AlertDescription className="flex items-center gap-2">
              <span>⚠️</span> {error}
            </AlertDescription>
          </Alert>
        )}

        <div className="bg-muted rounded-lg p-4 overflow-x-auto">
          <pre className="text-xs text-muted-foreground font-mono">{`const result = await rpc.greet({ name: "${name || "World"}" });
// Returns: "${greeting || "Hello, World! 👋"}"`}</pre>
        </div>
      </CardContent>
    </Card>
  );
}

function HookDemo() {
  const [name, setName] = useState("World");
  const { data, isLoading, error, refetch } = useQuery({
    ...orpc.greet.queryOptions({ input: { name } }),
    enabled: name.length > 0,
  });

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">useGreet Hook</CardTitle>
          <Badge className="bg-violet-500/20 text-violet-400 border-violet-500/30">
            Reactive
          </Badge>
        </div>
        <CardDescription>
          Reactive query that automatically refetches when input changes
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Type to greet..."
            className="flex-1"
          />
          <Button
            variant="secondary"
            onClick={() => refetch()}
            disabled={isLoading}
          >
            Refetch
          </Button>
        </div>

        <Alert
          className={
            isLoading
              ? "border-muted"
              : error
                ? "border-destructive bg-destructive/10"
                : "border-green-500/30 bg-green-500/10"
          }
        >
          <AlertDescription className="flex items-center gap-2">
            {isLoading && (
              <>
                <Spinner className="size-4" /> Loading...
              </>
            )}
            {error && (
              <>
                <span className="text-destructive">⚠️</span>{" "}
                <span className="text-destructive">{error.message}</span>
              </>
            )}
            {data && !isLoading && (
              <>
                <span className="text-green-500">✨</span>{" "}
                <span className="text-green-500">{data}</span>
              </>
            )}
          </AlertDescription>
        </Alert>

        <div className="bg-muted rounded-lg p-4 overflow-x-auto">
          <pre className="text-xs text-muted-foreground font-mono">{`const { data, isLoading, error, refetch } = useGreet(
  { name: "${name}" },
  { enabled: ${name.length > 0} }
);`}</pre>
        </div>
      </CardContent>
    </Card>
  );
}

function ValidationDemo() {
  const [name, setName] = useState("");
  const { data, isLoading, error } = useQuery({
    ...orpc.greet.queryOptions({ input: { name } }),
    enabled: name.length > 0,
  });

  const getValidationState = () => {
    if (name.length === 0)
      return { message: "Name is required", type: "info" as const };
    if (name.length < 2)
      return { message: "Name too short", type: "warning" as const };
    return { message: "Valid name", type: "success" as const };
  };

  const validation = getValidationState();
  const inputClass = {
    info: "",
    warning: "border-yellow-500 focus-visible:ring-yellow-500",
    success: "border-green-500 focus-visible:ring-green-500",
  }[validation.type];

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">Input Validation</CardTitle>
          <Badge variant="outline">Backend</Badge>
        </div>
        <CardDescription>
          The backend validates that name is not empty
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Try empty or short names..."
          className={inputClass}
        />

        <div
          className={`text-xs px-3 py-2 rounded-md ${
            validation.type === "info"
              ? "bg-blue-500/10 text-blue-400"
              : validation.type === "warning"
                ? "bg-yellow-500/10 text-yellow-400"
                : "bg-green-500/10 text-green-400"
          }`}
        >
          {validation.message}
        </div>

        <Alert
          className={
            isLoading
              ? "border-muted"
              : error
                ? "border-destructive bg-destructive/10"
                : data
                  ? "border-green-500/30 bg-green-500/10"
                  : "border-muted"
          }
        >
          <AlertDescription className="flex items-center gap-2">
            {isLoading && (
              <>
                <Spinner className="size-4" /> Validating...
              </>
            )}
            {error && (
              <>
                <span className="text-destructive">❌</span>{" "}
                <span className="text-destructive">{error.message}</span>
              </>
            )}
            {data && !isLoading && (
              <>
                <span className="text-green-500">✅</span>{" "}
                <span className="text-green-500">{data}</span>
              </>
            )}
            {!data && !isLoading && !error && (
              <span className="text-muted-foreground">
                Enter a name to test
              </span>
            )}
          </AlertDescription>
        </Alert>
      </CardContent>
    </Card>
  );
}

function GreetPage() {
  return (
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header>
        <h1 className="text-3xl font-bold mb-2">👋 Greet</h1>
        <p className="text-muted-foreground">
          Simple query example demonstrating RPC calls and React hooks
        </p>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
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
