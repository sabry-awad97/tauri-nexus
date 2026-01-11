import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { subscribe } from "../../lib/rpc";
import { useSubscription } from "../../lib/rpc/hooks";
import type { StockPrice } from "../../rpc/contract";

const AVAILABLE_SYMBOLS = ["AAPL", "GOOGL", "MSFT", "AMZN", "TSLA"];

function StockCard({
  symbol,
  price,
  isSelected,
  onToggle,
}: {
  symbol: string;
  price: StockPrice | undefined;
  isSelected: boolean;
  onToggle: () => void;
}) {
  const changeClass = price
    ? price.change >= 0
      ? "positive"
      : "negative"
    : "";

  return (
    <div
      className={`stock-card ${isSelected ? "selected" : ""} ${changeClass}`}
      onClick={onToggle}
    >
      <div className="stock-header">
        <span className="stock-symbol">{symbol}</span>
        <input
          type="checkbox"
          checked={isSelected}
          onChange={onToggle}
          onClick={(e) => e.stopPropagation()}
        />
      </div>

      {price ? (
        <>
          <div className="stock-price">${price.price.toFixed(2)}</div>
          <div className={`stock-change ${changeClass}`}>
            <span className="change-value">
              {price.change >= 0 ? "+" : ""}
              {price.change.toFixed(2)}
            </span>
            <span className="change-percent">
              ({price.changePercent >= 0 ? "+" : ""}
              {price.changePercent.toFixed(2)}%)
            </span>
          </div>
          <div className="stock-time">
            {new Date(price.timestamp).toLocaleTimeString()}
          </div>
        </>
      ) : (
        <div className="stock-placeholder">
          {isSelected ? "Waiting..." : "Click to track"}
        </div>
      )}
    </div>
  );
}

function PriceChart({ history }: { history: StockPrice[] }) {
  if (history.length < 2) return null;

  const prices = history.map((h) => h.price);
  const min = Math.min(...prices);
  const max = Math.max(...prices);
  const range = max - min || 1;

  const points = history
    .map((h, i) => {
      const x = (i / (history.length - 1)) * 100;
      const y = 100 - ((h.price - min) / range) * 100;
      return `${x},${y}`;
    })
    .join(" ");

  const lastPrice = history[history.length - 1];
  const firstPrice = history[0];
  const isUp = lastPrice.price >= firstPrice.price;

  return (
    <div className="price-chart">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        <polyline
          points={points}
          fill="none"
          stroke={isUp ? "#22c55e" : "#ef4444"}
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <div className="chart-labels">
        <span>${min.toFixed(2)}</span>
        <span>${max.toFixed(2)}</span>
      </div>
    </div>
  );
}

function StocksPage() {
  const [selectedSymbols, setSelectedSymbols] = useState<string[]>([
    "AAPL",
    "GOOGL",
    "MSFT",
  ]);
  const [prices, setPrices] = useState<Map<string, StockPrice>>(new Map());
  const [history, setHistory] = useState<Map<string, StockPrice[]>>(new Map());

  const { isConnected, error } = useSubscription<StockPrice>(
    async () =>
      subscribe<StockPrice>("stream.stocks", { symbols: selectedSymbols }),
    [selectedSymbols.join(",")],
    {
      enabled: selectedSymbols.length > 0,
      onEvent: (price) => {
        setPrices((prev) => new Map(prev).set(price.symbol, price));
        setHistory((prev) => {
          const newHistory = new Map(prev);
          const symbolHistory = newHistory.get(price.symbol) || [];
          newHistory.set(price.symbol, [...symbolHistory.slice(-29), price]);
          return newHistory;
        });
      },
    },
  );

  const toggleSymbol = (symbol: string) => {
    setSelectedSymbols((prev) =>
      prev.includes(symbol)
        ? prev.filter((s) => s !== symbol)
        : [...prev, symbol],
    );
  };

  return (
    <div className="page stream-page stocks-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">📈 Stock Ticker</h1>
          <p className="page-subtitle">
            Real-time simulated stock prices with live updates
          </p>
        </div>
        <div
          className={`connection-status ${isConnected ? "connected" : "disconnected"}`}
        >
          <span className="status-dot" />
          {isConnected
            ? "Live"
            : selectedSymbols.length === 0
              ? "Select stocks"
              : "Connecting..."}
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <span>⚠️</span> {error.message}
        </div>
      )}

      <div className="stocks-grid">
        {AVAILABLE_SYMBOLS.map((symbol) => (
          <StockCard
            key={symbol}
            symbol={symbol}
            price={prices.get(symbol)}
            isSelected={selectedSymbols.includes(symbol)}
            onToggle={() => toggleSymbol(symbol)}
          />
        ))}
      </div>

      {selectedSymbols.length > 0 && (
        <div className="charts-section">
          <h2>Price History</h2>
          <div className="charts-grid">
            {selectedSymbols.map((symbol) => {
              const symbolHistory = history.get(symbol) || [];
              const currentPrice = prices.get(symbol);

              return (
                <div key={symbol} className="chart-card">
                  <div className="chart-header">
                    <span className="chart-symbol">{symbol}</span>
                    {currentPrice && (
                      <span
                        className={`chart-price ${currentPrice.change >= 0 ? "positive" : "negative"}`}
                      >
                        ${currentPrice.price.toFixed(2)}
                      </span>
                    )}
                  </div>
                  <PriceChart history={symbolHistory} />
                  <div className="chart-footer">
                    {symbolHistory.length} data points
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      <div className="code-example">
        <h3>Code Example</h3>
        <pre>{`const { isConnected } = useSubscription<StockPrice>(
  async () => subscribe('stream.stocks', { 
    symbols: ${JSON.stringify(selectedSymbols)} 
  }),
  [symbols],
  {
    onEvent: (price) => {
      console.log(\`\${price.symbol}: $\${price.price}\`);
    },
  }
);`}</pre>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/streams/stocks")({
  component: StocksPage,
});
