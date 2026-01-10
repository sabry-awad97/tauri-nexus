import { createFileRoute } from '@tanstack/react-router';
import { useState, useRef } from 'react';
import { subscribe } from '../../lib/rpc';
import type { CounterEvent } from '../../rpc/contract';

function CounterPage() {
  const [count, setCount] = useState<number | null>(null);
  const [events, setEvents] = useState<CounterEvent[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [config, setConfig] = useState({ start: 0, maxCount: 20, intervalMs: 500 });
  const cancelRef = useRef<(() => Promise<void>) | null>(null);

  const startCounter = async () => {
    setIsRunning(true);
    setCount(null);
    setEvents([]);
    
    try {
      const stream = await subscribe<CounterEvent>('stream.counter', config);
      
      cancelRef.current = async () => {
        await stream.return();
      };
      
      for await (const event of stream) {
        setCount(event.count);
        setEvents(prev => [...prev.slice(-9), event]);
      }
    } catch (err) {
      console.error('Counter error:', err);
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

  const progress = count !== null 
    ? ((count - config.start + 1) / config.maxCount) * 100 
    : 0;

  return (
    <div className="page stream-page counter-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">🔢 Counter Stream</h1>
          <p className="page-subtitle">
            Simple incrementing counter demonstrating basic streaming
          </p>
        </div>
        <div className={`connection-status ${isRunning ? 'connected' : 'disconnected'}`}>
          <span className="status-dot" />
          {isRunning ? 'Streaming' : 'Stopped'}
        </div>
      </header>

      <div className="stream-layout">
        <div className="stream-main">
          <div className="counter-display-card">
            <div className="counter-value">
              {count !== null ? count : '—'}
            </div>
            <div className="counter-progress">
              <div className="progress-bar" style={{ width: `${progress}%` }} />
            </div>
            <div className="counter-info">
              {count !== null ? (
                <span>{count - config.start + 1} of {config.maxCount} events</span>
              ) : (
                <span>Ready to start</span>
              )}
            </div>
          </div>

          <div className="stream-controls">
            <button 
              onClick={startCounter} 
              disabled={isRunning}
              className="control-btn start"
            >
              <span>▶️</span> Start
            </button>
            <button 
              onClick={stopCounter} 
              disabled={!isRunning}
              className="control-btn stop"
            >
              <span>⏹️</span> Stop
            </button>
          </div>
        </div>

        <aside className="stream-sidebar">
          <div className="config-panel">
            <h3>Configuration</h3>
            
            <div className="config-field">
              <label>Start Value</label>
              <input
                type="number"
                value={config.start}
                onChange={(e) => setConfig(c => ({ ...c, start: parseInt(e.target.value) || 0 }))}
                disabled={isRunning}
              />
            </div>

            <div className="config-field">
              <label>Max Count</label>
              <input
                type="number"
                value={config.maxCount}
                onChange={(e) => setConfig(c => ({ ...c, maxCount: parseInt(e.target.value) || 10 }))}
                disabled={isRunning}
                min={1}
                max={100}
              />
            </div>

            <div className="config-field">
              <label>Interval (ms)</label>
              <input
                type="number"
                value={config.intervalMs}
                onChange={(e) => setConfig(c => ({ ...c, intervalMs: parseInt(e.target.value) || 500 }))}
                disabled={isRunning}
                min={100}
                max={5000}
                step={100}
              />
            </div>
          </div>

          <div className="events-panel">
            <h3>Recent Events</h3>
            <div className="events-list">
              {events.length === 0 ? (
                <div className="no-events">No events yet</div>
              ) : (
                events.map((event, i) => (
                  <div key={i} className="event-item">
                    <span className="event-count">{event.count}</span>
                    <span className="event-time">
                      {new Date(event.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </aside>
      </div>

      <div className="code-example">
        <h3>Code Example</h3>
        <pre>{`const stream = await subscribe<CounterEvent>('stream.counter', {
  start: ${config.start},
  maxCount: ${config.maxCount},
  intervalMs: ${config.intervalMs}
});

for await (const event of stream) {
  console.log(event.count); // ${count ?? 0}, ${(count ?? 0) + 1}, ${(count ?? 0) + 2}...
}`}</pre>
      </div>
    </div>
  );
}

export const Route = createFileRoute('/streams/counter')({
  component: CounterPage,
});
