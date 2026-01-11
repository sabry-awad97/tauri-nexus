import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { subscribe, useSubscription } from "@tauri-nexus/rpc-react";

function DigitalClock({ time }: { time: string }) {
  if (!time) return <div className="clock-placeholder">--:--:--</div>;

  const date = new Date(time);
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  const seconds = date.getSeconds().toString().padStart(2, "0");

  return (
    <div className="digital-clock">
      <div className="clock-segment">
        <span className="clock-value">{hours}</span>
        <span className="clock-label">Hours</span>
      </div>
      <span className="clock-separator">:</span>
      <div className="clock-segment">
        <span className="clock-value">{minutes}</span>
        <span className="clock-label">Minutes</span>
      </div>
      <span className="clock-separator">:</span>
      <div className="clock-segment">
        <span className="clock-value">{seconds}</span>
        <span className="clock-label">Seconds</span>
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
    <div className="analog-clock">
      <div className="clock-face">
        {[...Array(12)].map((_, i) => (
          <div
            key={i}
            className="clock-mark"
            style={{ transform: `rotate(${i * 30}deg)` }}
          />
        ))}
        <div
          className="clock-hand hour"
          style={{ transform: `rotate(${hourDeg}deg)` }}
        />
        <div
          className="clock-hand minute"
          style={{ transform: `rotate(${minuteDeg}deg)` }}
        />
        <div
          className="clock-hand second"
          style={{ transform: `rotate(${secondDeg}deg)` }}
        />
        <div className="clock-center" />
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
    <div className="world-clocks">
      <h3>World Clocks</h3>
      <div className="world-clocks-grid">
        {timezones.map((tz) => {
          const localTime = new Date(
            serverDate.getTime() + tz.offset * 60 * 60 * 1000,
          );
          return (
            <div key={tz.name} className="world-clock-item">
              <span className="city-name">{tz.name}</span>
              <span className="city-time">
                {localTime.toLocaleTimeString("en-US", {
                  hour: "2-digit",
                  minute: "2-digit",
                  hour12: true,
                })}
              </span>
              <span className="city-offset">
                UTC{tz.offset >= 0 ? "+" : ""}
                {tz.offset}
              </span>
            </div>
          );
        })}
      </div>
    </div>
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
    <div className="page stream-page time-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">⏰ Server Time</h1>
          <p className="page-subtitle">
            Real-time server clock synchronized every second
          </p>
        </div>
        <div
          className={`connection-status ${isConnected ? "connected" : "disconnected"}`}
        >
          <span className="status-dot" />
          {isConnected ? "Synced" : "Connecting..."}
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <span>⚠️</span> {error.message}
        </div>
      )}

      <div className="time-layout">
        <div className="clocks-section">
          <div className="clock-card digital">
            <h3>Digital</h3>
            <DigitalClock time={time} />
          </div>

          <div className="clock-card analog">
            <h3>Analog</h3>
            <AnalogClock time={time} />
          </div>
        </div>

        <div className="time-info-section">
          <div className="info-card">
            <h3>Server Info</h3>
            <div className="info-grid">
              <div className="info-item">
                <span className="info-label">Date</span>
                <span className="info-value">
                  {serverDate?.toLocaleDateString("en-US", {
                    weekday: "long",
                    year: "numeric",
                    month: "long",
                    day: "numeric",
                  }) || "—"}
                </span>
              </div>
              <div className="info-item">
                <span className="info-label">ISO 8601</span>
                <span className="info-value mono">{time || "—"}</span>
              </div>
              <div className="info-item">
                <span className="info-label">Unix Timestamp</span>
                <span className="info-value mono">
                  {serverDate?.getTime() || "—"}
                </span>
              </div>
              <div className="info-item">
                <span className="info-label">Ticks Received</span>
                <span className="info-value">{tickCount}</span>
              </div>
              <div className="info-item">
                <span className="info-label">Latency</span>
                <span
                  className={`info-value ${latency && latency > 100 ? "warning" : ""}`}
                >
                  {latency !== null ? `${latency}ms` : "—"}
                </span>
              </div>
            </div>
          </div>

          <WorldClocks serverTime={time} />
        </div>
      </div>

      <div className="code-example">
        <h3>Code Example</h3>
        <pre>{`const { isConnected } = useSubscription<string>(
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
    </div>
  );
}

export const Route = createFileRoute("/streams/time")({
  component: TimePage,
});
