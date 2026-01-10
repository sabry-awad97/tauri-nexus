// =============================================================================
// Subscription Demo Component
// =============================================================================
//
// Demonstrates the Event Iterator / Streaming functionality

import { useState, useEffect, useRef } from "react";
import { subscribe } from "../lib/rpc";
import { useSubscription } from "../lib/rpc/hooks";
import type { CounterEvent, StockPrice, ChatMessage } from "../rpc/contract";

// =============================================================================
// Counter Demo - Simple incrementing counter
// =============================================================================

export function CounterDemo() {
  const [count, setCount] = useState<number | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const cancelRef = useRef<(() => Promise<void>) | null>(null);

  const startCounter = async () => {
    setIsRunning(true);
    setCount(null);

    try {
      const stream = await subscribe<CounterEvent>("stream.counter", {
        start: 0,
        maxCount: 20,
        intervalMs: 500,
      });

      // Store cancel function
      cancelRef.current = async () => {
        await stream.return();
      };

      for await (const event of stream) {
        setCount(event.count);
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

  return (
    <div className="demo-card">
      <h3>🔢 Counter Stream</h3>
      <div className="counter-display">{count !== null ? count : "—"}</div>
      <div className="button-group">
        <button onClick={startCounter} disabled={isRunning}>
          Start
        </button>
        <button onClick={stopCounter} disabled={!isRunning}>
          Stop
        </button>
      </div>
      <p className="status">{isRunning ? "🟢 Running" : "⚪ Stopped"}</p>
    </div>
  );
}

// =============================================================================
// Stock Ticker Demo - Real-time stock prices
// =============================================================================

export function StockTickerDemo() {
  const [prices, setPrices] = useState<Map<string, StockPrice>>(new Map());
  const { isConnected } = useSubscription<StockPrice>(
    async () =>
      subscribe<StockPrice>("stream.stocks", {
        symbols: ["AAPL", "GOOGL", "MSFT"],
      }),
    [],
    {
      onEvent: (price) => {
        setPrices((prev) => new Map(prev).set(price.symbol, price));
      },
    },
  );

  return (
    <div className="demo-card">
      <h3>📈 Stock Ticker</h3>
      <p className="status">{isConnected ? "🟢 Live" : "🔴 Disconnected"}</p>
      <div className="stock-list">
        {["AAPL", "GOOGL", "MSFT"].map((symbol) => {
          const price = prices.get(symbol);
          return (
            <div key={symbol} className="stock-item">
              <span className="symbol">{symbol}</span>
              <span className="price">${price?.price.toFixed(2) ?? "—"}</span>
              <span
                className={`change ${(price?.change ?? 0) >= 0 ? "up" : "down"}`}
              >
                {price
                  ? `${price.change >= 0 ? "+" : ""}${price.change.toFixed(2)}`
                  : "—"}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// =============================================================================
// Chat Demo - Real-time chat messages
// =============================================================================

export function ChatDemo() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const { isConnected } = useSubscription<ChatMessage>(
    async () => subscribe<ChatMessage>("stream.chat", { roomId: "general" }),
    [],
    {
      onEvent: (message) => {
        setMessages((prev) => [...prev.slice(-19), message]); // Keep last 20
      },
      autoReconnect: true,
    },
  );

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="demo-card chat-card">
      <h3>💬 Chat Room</h3>
      <p className="status">
        {isConnected ? "🟢 Connected" : "🔴 Reconnecting..."}
      </p>
      <div className="chat-messages">
        {messages.map((msg) => (
          <div key={msg.id} className="chat-message">
            <span className="user">{msg.userId}</span>
            <span className="text">{msg.text}</span>
            <span className="time">
              {new Date(msg.timestamp).toLocaleTimeString()}
            </span>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>
    </div>
  );
}

// =============================================================================
// Time Demo - Current time stream
// =============================================================================

export function TimeDemo() {
  const [time, setTime] = useState<string>("");

  const { isConnected } = useSubscription<string>(
    async () => subscribe<string>("stream.time", {}),
    [],
    {
      onEvent: (t) => setTime(t),
    },
  );

  const formatTime = (iso: string) => {
    if (!iso) return "—";
    const date = new Date(iso);
    return date.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  return (
    <div className="demo-card">
      <h3>⏰ Server Time</h3>
      <div className="time-display">{formatTime(time)}</div>
      <p className="status">{isConnected ? "🟢 Synced" : "🔴 Disconnected"}</p>
    </div>
  );
}

// =============================================================================
// Main Demo Component
// =============================================================================

export function SubscriptionDemo() {
  return (
    <div className="subscription-demo">
      <h2>Event Iterator / Streaming Demo</h2>
      <p className="description">
        These examples demonstrate real-time streaming from Rust to TypeScript
        using the Event Iterator pattern (SSE-style).
      </p>

      <div className="demo-grid">
        <CounterDemo />
        <TimeDemo />
        <StockTickerDemo />
        <ChatDemo />
      </div>

      <style>{`
        .subscription-demo {
          padding: 20px;
          max-width: 1200px;
          margin: 0 auto;
        }
        
        .subscription-demo h2 {
          margin-bottom: 8px;
        }
        
        .description {
          color: #666;
          margin-bottom: 24px;
        }
        
        .demo-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
          gap: 20px;
        }
        
        .demo-card {
          background: #f8f9fa;
          border-radius: 12px;
          padding: 20px;
          box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
        
        .demo-card h3 {
          margin: 0 0 12px 0;
          font-size: 18px;
        }
        
        .status {
          font-size: 12px;
          color: #666;
          margin: 8px 0;
        }
        
        .counter-display, .time-display {
          font-size: 48px;
          font-weight: bold;
          text-align: center;
          padding: 20px;
          font-family: monospace;
        }
        
        .button-group {
          display: flex;
          gap: 8px;
          justify-content: center;
        }
        
        .button-group button {
          padding: 8px 16px;
          border: none;
          border-radius: 6px;
          cursor: pointer;
          font-weight: 500;
        }
        
        .button-group button:first-child {
          background: #4CAF50;
          color: white;
        }
        
        .button-group button:last-child {
          background: #f44336;
          color: white;
        }
        
        .button-group button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        
        .stock-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        
        .stock-item {
          display: flex;
          justify-content: space-between;
          padding: 8px 12px;
          background: white;
          border-radius: 6px;
        }
        
        .stock-item .symbol {
          font-weight: bold;
        }
        
        .stock-item .price {
          font-family: monospace;
        }
        
        .stock-item .change {
          font-family: monospace;
          font-size: 14px;
        }
        
        .stock-item .change.up {
          color: #4CAF50;
        }
        
        .stock-item .change.down {
          color: #f44336;
        }
        
        .chat-card {
          grid-column: span 2;
        }
        
        .chat-messages {
          height: 200px;
          overflow-y: auto;
          background: white;
          border-radius: 6px;
          padding: 12px;
        }
        
        .chat-message {
          display: flex;
          gap: 8px;
          padding: 4px 0;
          border-bottom: 1px solid #eee;
        }
        
        .chat-message .user {
          font-weight: bold;
          color: #1976d2;
          min-width: 80px;
        }
        
        .chat-message .text {
          flex: 1;
        }
        
        .chat-message .time {
          font-size: 12px;
          color: #999;
        }
        
        @media (max-width: 768px) {
          .chat-card {
            grid-column: span 1;
          }
        }
      `}</style>
    </div>
  );
}

export default SubscriptionDemo;
