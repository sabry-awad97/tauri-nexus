import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { subscribe, useSubscription } from "@tauri-nexus/rpc-react";
import type { StockPrice } from "../../rpc/contract";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";

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
  const isPositive = price ? price.change >= 0 : true;

  return (
    <Card
      className={`cursor-pointer transition-all hover:border-muted-foreground/50 ${
        isSelected ? "border-primary" : ""
      } ${price ? (isPositive ? "border-l-4 border-l-green-500" : "border-l-4 border-l-red-500") : ""}`}
      onClick={onToggle}
    >
      <CardContent className="p-5">
        <div className="flex items-center justify-between mb-3">
          <span className="text-lg font-bold">{symbol}</span>
          <Checkbox
            checked={isSelected}
            onCheckedChange={onToggle}
            onClick={(e) => e.stopPropagation()}
          />
        </div>

        {price ? (
          <>
            <div className="text-3xl font-bold mb-2">
              ${price.price.toFixed(2)}
            </div>
            <div
              className={`flex gap-2 text-sm mb-2 ${isPositive ? "text-green-500" : "text-red-500"}`}
            >
              <span>
                {isPositive ? "+" : ""}
                {price.change.toFixed(2)}
              </span>
              <span>
                ({isPositive ? "+" : ""}
                {price.changePercent.toFixed(2)}%)
              </span>
            </div>
            <p className="text-xs text-muted-foreground">
              {new Date(price.timestamp).toLocaleTimeString()}
            </p>
          </>
        ) : (
          <p className="text-sm text-muted-foreground py-4">
            {isSelected ? "Waiting..." : "Click to track"}
          </p>
        )}
      </CardContent>
    </Card>
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
    <div className="h-24 relative mb-2">
      <svg
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        className="w-full h-full"
      >
        <polyline
          points={points}
          fill="none"
          stroke={isUp ? "#22c55e" : "#ef4444"}
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <div className="flex justify-between text-[10px] text-muted-foreground">
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
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">📈 Stock Ticker</h1>
          <p className="text-muted-foreground">
            Real-time simulated stock prices with live updates
          </p>
        </div>
        <Badge
          variant={isConnected ? "default" : "secondary"}
          className="gap-2"
        >
          <span
            className={`size-2 rounded-full ${isConnected ? "bg-green-500 animate-pulse" : "bg-muted-foreground"}`}
          />
          {isConnected
            ? "Live"
            : selectedSymbols.length === 0
              ? "Select stocks"
              : "Connecting..."}
        </Badge>
      </header>

      {error && (
        <div className="bg-destructive/10 border border-destructive/30 rounded-lg p-4 flex items-center gap-2 text-destructive">
          <span>⚠️</span> {error.message}
        </div>
      )}

      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-4">
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
        <section>
          <h2 className="text-lg font-semibold mb-4">Price History</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {selectedSymbols.map((symbol) => {
              const symbolHistory = history.get(symbol) || [];
              const currentPrice = prices.get(symbol);

              return (
                <Card key={symbol}>
                  <CardHeader className="pb-2">
                    <div className="flex justify-between items-center">
                      <CardTitle className="text-sm font-semibold">
                        {symbol}
                      </CardTitle>
                      {currentPrice && (
                        <span
                          className={`font-mono font-semibold ${currentPrice.change >= 0 ? "text-green-500" : "text-red-500"}`}
                        >
                          ${currentPrice.price.toFixed(2)}
                        </span>
                      )}
                    </div>
                  </CardHeader>
                  <CardContent>
                    <PriceChart history={symbolHistory} />
                    <p className="text-xs text-muted-foreground text-center">
                      {symbolHistory.length} data points
                    </p>
                  </CardContent>
                </Card>
              );
            })}
          </div>
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
            <pre className="text-xs font-mono text-muted-foreground">{`const { isConnected } = useSubscription<StockPrice>(
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
        </CardContent>
      </Card>
    </div>
  );
}

export const Route = createFileRoute("/streams/stocks")({
  component: StocksPage,
});
