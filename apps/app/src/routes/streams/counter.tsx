import { createFileRoute } from "@tanstack/react-router";
import { useState, useRef } from "react";
import { subscribe } from "@tauri-nexus/rpc-react";
import type { CounterEvent } from "../../rpc/contract";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";

function CounterPage() {
  const [count, setCount] = useState<number | null>(null);
  const [events, setEvents] = useState<CounterEvent[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [config, setConfig] = useState({
    start: 0,
    maxCount: 20,
    intervalMs: 500,
  });
  const cancelRef = useRef<(() => Promise<void>) | null>(null);

  const startCounter = async () => {
    setIsRunning(true);
    setCount(null);
    setEvents([]);

    try {
      const stream = await subscribe<CounterEvent>("stream.counter", config);

      cancelRef.current = async () => {
        await stream.return();
      };

      for await (const event of stream) {
        setCount(event.count);
        setEvents((prev) => [...prev.slice(-9), event]);
      }
    } catch (err) {
      console.error("Counter error:", err);
    } finally {
      setIsRunning(false);
      cancelRef.current = null;
    }
  };

  const stopCounter = async () => {
    if (cancelRef.current) {
      await cancelRef.current();
    }
  };

  const progress =
    count !== null ? ((count - config.start + 1) / config.maxCount) * 100 : 0;

  return (
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">🔢 Counter Stream</h1>
          <p className="text-muted-foreground">
            Simple incrementing counter demonstrating basic streaming
          </p>
        </div>
        <Badge variant={isRunning ? "default" : "secondary"} className="gap-2">
          <span
            className={`size-2 rounded-full ${isRunning ? "bg-green-500 animate-pulse" : "bg-muted-foreground"}`}
          />
          {isRunning ? "Streaming" : "Stopped"}
        </Badge>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-6">
        <div className="space-y-6">
          <Card>
            <CardContent className="p-12 text-center">
              <div className="text-8xl font-bold font-mono text-primary mb-6">
                {count !== null ? count : "—"}
              </div>
              <Progress value={progress} className="h-2 mb-4" />
              <p className="text-sm text-muted-foreground">
                {count !== null ? (
                  <>
                    {count - config.start + 1} of {config.maxCount} events
                  </>
                ) : (
                  "Ready to start"
                )}
              </p>
            </CardContent>
          </Card>

          <div className="flex justify-center gap-3">
            <Button
              size="lg"
              onClick={startCounter}
              disabled={isRunning}
              className="bg-green-600 hover:bg-green-700"
            >
              <span className="mr-2">▶️</span> Start
            </Button>
            <Button
              size="lg"
              variant="destructive"
              onClick={stopCounter}
              disabled={!isRunning}
            >
              <span className="mr-2">⏹️</span> Stop
            </Button>
          </div>
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Configuration</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label className="text-xs">Start Value</Label>
                <Input
                  type="number"
                  value={config.start}
                  onChange={(e) =>
                    setConfig((c) => ({
                      ...c,
                      start: parseInt(e.target.value) || 0,
                    }))
                  }
                  disabled={isRunning}
                />
              </div>
              <div className="space-y-2">
                <Label className="text-xs">Max Count</Label>
                <Input
                  type="number"
                  value={config.maxCount}
                  onChange={(e) =>
                    setConfig((c) => ({
                      ...c,
                      maxCount: parseInt(e.target.value) || 10,
                    }))
                  }
                  disabled={isRunning}
                  min={1}
                  max={100}
                />
              </div>
              <div className="space-y-2">
                <Label className="text-xs">Interval (ms)</Label>
                <Input
                  type="number"
                  value={config.intervalMs}
                  onChange={(e) =>
                    setConfig((c) => ({
                      ...c,
                      intervalMs: parseInt(e.target.value) || 500,
                    }))
                  }
                  disabled={isRunning}
                  min={100}
                  max={5000}
                  step={100}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Recent Events</CardTitle>
            </CardHeader>
            <CardContent>
              <ScrollArea className="h-[200px]">
                {events.length === 0 ? (
                  <p className="text-center text-sm text-muted-foreground py-8">
                    No events yet
                  </p>
                ) : (
                  <div className="space-y-2">
                    {events.map((event, i) => (
                      <div
                        key={i}
                        className="flex justify-between items-center p-2 rounded bg-muted/50 text-sm"
                      >
                        <span className="font-mono font-semibold">
                          {event.count}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {new Date(event.timestamp).toLocaleTimeString()}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </ScrollArea>
            </CardContent>
          </Card>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm text-muted-foreground">
            Code Example
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="bg-muted rounded-lg p-4 overflow-x-auto">
            <pre className="text-xs font-mono text-muted-foreground">{`const stream = await subscribe<CounterEvent>('stream.counter', {
  start: ${config.start},
  maxCount: ${config.maxCount},
  intervalMs: ${config.intervalMs}
});

for await (const event of stream) {
  console.log(event.count); // ${count ?? 0}, ${(count ?? 0) + 1}, ${(count ?? 0) + 2}...
}`}</pre>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export const Route = createFileRoute("/streams/counter")({
  component: CounterPage,
});
