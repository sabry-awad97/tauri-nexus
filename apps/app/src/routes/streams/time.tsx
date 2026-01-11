import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { subscribe, useSubscription } from "@tauri-nexus/rpc-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

function DigitalClock({ time }: { time: string }) {
  if (!time)
    return (
      <div className="text-6xl font-bold font-mono text-muted-foreground text-center">
        --:--:--
      </div>
    );

  const date = new Date(time);
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  const seconds = date.getSeconds().toString().padStart(2, "0");

  return (
    <div className="flex justify-center items-center gap-2">
      <div className="text-center">
        <span className="block text-5xl font-bold font-mono text-primary bg-muted px-4 py-3 rounded-xl">
          {hours}
        </span>
        <span className="text-[10px] text-muted-foreground uppercase tracking-wider mt-2 block">
          Hours
        </span>
      </div>
      <span className="text-4xl font-bold text-muted-foreground mb-6">:</span>
      <div className="text-center">
        <span className="block text-5xl font-bold font-mono text-primary bg-muted px-4 py-3 rounded-xl">
          {minutes}
        </span>
        <span className="text-[10px] text-muted-foreground uppercase tracking-wider mt-2 block">
          Minutes
        </span>
      </div>
      <span className="text-4xl font-bold text-muted-foreground mb-6">:</span>
      <div className="text-center">
        <span className="block text-5xl font-bold font-mono text-primary bg-muted px-4 py-3 rounded-xl">
          {seconds}
        </span>
        <span className="text-[10px] text-muted-foreground uppercase tracking-wider mt-2 block">
          Seconds
        </span>
      </div>
    </div>
  );
}

function AnalogClock({ time }: { time: string }) {
  if (!time) return null;

  const date = new Date(time);
  const seconds = date.getSeconds();
  const minutes = date.getMinutes();
  const hours = date.getHours() % 12;

  const secondDeg = (seconds / 60) * 360;
  const minuteDeg = ((minutes + seconds / 60) / 60) * 360;
  const hourDeg = ((hours + minutes / 60) / 12) * 360;

  return (
    <div className="flex justify-center">
      <div className="relative size-48 rounded-full bg-muted border-4 border-border">
        {[...Array(12)].map((_, i) => (
          <div
            key={i}
            className="absolute w-0.5 bg-muted-foreground"
            style={{
              height: i % 3 === 0 ? "16px" : "10px",
              top: "10px",
              left: "50%",
              transformOrigin: "0 84px",
              transform: `rotate(${i * 30}deg)`,
            }}
          />
        ))}
        <div
          className="absolute bottom-1/2 left-1/2 w-1 bg-foreground rounded origin-bottom"
          style={{
            height: "50px",
            marginLeft: "-2px",
            transform: `rotate(${hourDeg}deg)`,
          }}
        />
        <div
          className="absolute bottom-1/2 left-1/2 w-0.5 bg-muted-foreground rounded origin-bottom"
          style={{
            height: "70px",
            marginLeft: "-1px",
            transform: `rotate(${minuteDeg}deg)`,
          }}
        />
        <div
          className="absolute bottom-1/2 left-1/2 w-0.5 bg-red-500 rounded origin-bottom"
          style={{
            height: "80px",
            marginLeft: "-1px",
            transform: `rotate(${secondDeg}deg)`,
          }}
        />
        <div className="absolute top-1/2 left-1/2 size-3 bg-primary rounded-full -translate-x-1/2 -translate-y-1/2" />
      </div>
    </div>
  );
}

function WorldClocks({ serverTime }: { serverTime: string }) {
  if (!serverTime) return null;

  const serverDate = new Date(serverTime);

  const timezones = [
    { name: "New York", offset: -5 },
    { name: "London", offset: 0 },
    { name: "Tokyo", offset: 9 },
    { name: "Sydney", offset: 11 },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">World Clocks</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-4">
          {timezones.map((tz) => {
            const localTime = new Date(
              serverDate.getTime() + tz.offset * 60 * 60 * 1000,
            );
            return (
              <div
                key={tz.name}
                className="text-center p-3 rounded-lg bg-muted/50"
              >
                <p className="text-sm font-medium">{tz.name}</p>
                <p className="text-lg font-mono">
                  {localTime.toLocaleTimeString("en-US", {
                    hour: "2-digit",
                    minute: "2-digit",
                    hour12: true,
                  })}
                </p>
                <p className="text-xs text-muted-foreground">
                  UTC{tz.offset >= 0 ? "+" : ""}
                  {tz.offset}
                </p>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

function TimePage() {
  const [time, setTime] = useState<string>("");
  const [tickCount, setTickCount] = useState(0);
  const [latency, setLatency] = useState<number | null>(null);

  const { isConnected, error } = useSubscription<string>(
    async () => subscribe<string>("stream.time", {}),
    [],
    {
      onEvent: (t) => {
        const now = Date.now();
        const serverTime = new Date(t).getTime();
        setLatency(now - serverTime);
        setTime(t);
        setTickCount((c) => c + 1);
      },
    },
  );

  const serverDate = time ? new Date(time) : null;

  return (
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">⏰ Server Time</h1>
          <p className="text-muted-foreground">
            Real-time server clock synchronized every second
          </p>
        </div>
        <Badge
          variant={isConnected ? "default" : "secondary"}
          className="gap-2"
        >
          <span
            className={`size-2 rounded-full ${isConnected ? "bg-green-500 animate-pulse" : "bg-muted-foreground"}`}
          />
          {isConnected ? "Synced" : "Connecting..."}
        </Badge>
      </header>

      {error && (
        <div className="bg-destructive/10 border border-destructive/30 rounded-lg p-4 flex items-center gap-2 text-destructive">
          <span>⚠️</span> {error.message}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-center text-muted-foreground">
                Digital
              </CardTitle>
            </CardHeader>
            <CardContent className="py-8">
              <DigitalClock time={time} />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-center text-muted-foreground">
                Analog
              </CardTitle>
            </CardHeader>
            <CardContent className="py-8">
              <AnalogClock time={time} />
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Server Info</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex justify-between items-center py-2 border-b">
                <span className="text-sm text-muted-foreground">Date</span>
                <span className="text-sm">
                  {serverDate?.toLocaleDateString("en-US", {
                    weekday: "long",
                    year: "numeric",
                    month: "long",
                    day: "numeric",
                  }) || "—"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b">
                <span className="text-sm text-muted-foreground">ISO 8601</span>
                <span className="text-sm font-mono">{time || "—"}</span>
              </div>
              <div className="flex justify-between items-center py-2 border-b">
                <span className="text-sm text-muted-foreground">
                  Unix Timestamp
                </span>
                <span className="text-sm font-mono">
                  {serverDate?.getTime() || "—"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b">
                <span className="text-sm text-muted-foreground">
                  Ticks Received
                </span>
                <span className="text-sm">{tickCount}</span>
              </div>
              <div className="flex justify-between items-center py-2">
                <span className="text-sm text-muted-foreground">Latency</span>
                <span
                  className={`text-sm ${latency && latency > 100 ? "text-yellow-500" : ""}`}
                >
                  {latency !== null ? `${latency}ms` : "—"}
                </span>
              </div>
            </CardContent>
          </Card>

          <WorldClocks serverTime={time} />
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
            <pre className="text-xs font-mono text-muted-foreground">{`const { isConnected } = useSubscription<string>(
  async () => subscribe('stream.time', {}),
  [],
  {
    onEvent: (isoTime) => {
      setTime(isoTime);
      // "${time || "2026-01-01T00:00:00.000Z"}"
    },
  }
);`}</pre>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export const Route = createFileRoute("/streams/time")({
  component: TimePage,
});
