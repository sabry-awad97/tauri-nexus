import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { useQuery } from "@tanstack/react-query";
import { RpcProvider, orpc } from "../rpc/contract";
import "../styles/global.css";

function HealthStatus() {
  const { data, isLoading, error } = useQuery({
    ...orpc.health.queryOptions(),
    refetchInterval: 30000,
  });

  if (isLoading)
    return <span className="health-dot loading" title="Checking..." />;
  if (error) return <span className="health-dot error" title="Disconnected" />;
  return (
    <span className="health-dot ok" title={`Connected v${data?.version}`} />
  );
}

function RootLayout() {
  return (
    <RpcProvider>
      <div className="app-layout">
        <nav className="sidebar">
          <div className="sidebar-header">
            <div className="logo">
              <span className="logo-icon">⚡</span>
              <span className="logo-text">Tauri RPC</span>
            </div>
            <HealthStatus />
          </div>

          <div className="nav-section">
            <span className="nav-label">Overview</span>
            <Link
              to="/"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">🏠</span>
              Dashboard
            </Link>
          </div>

          <div className="nav-section">
            <span className="nav-label">Queries</span>
            <Link
              to="/greet"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">👋</span>
              Greet
            </Link>
            <Link
              to="/users"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">👥</span>
              Users
            </Link>
          </div>

          <div className="nav-section">
            <span className="nav-label">Advanced</span>
            <Link
              to="/batch"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">📦</span>
              Batch
            </Link>
            <Link
              to="/advanced"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">🔧</span>
              Advanced
            </Link>
          </div>

          <div className="nav-section">
            <span className="nav-label">Subscriptions</span>
            <Link
              to="/streams/counter"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">🔢</span>
              Counter
            </Link>
            <Link
              to="/streams/stocks"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">📈</span>
              Stocks
            </Link>
            <Link
              to="/streams/chat"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">💬</span>
              Chat
            </Link>
            <Link
              to="/streams/time"
              className="nav-link"
              activeProps={{ className: "nav-link active" }}
            >
              <span className="nav-icon">⏰</span>
              Time
            </Link>
          </div>

          <div className="sidebar-footer">
            <span className="version">v0.1.0</span>
          </div>
        </nav>

        <main className="main-content">
          <Outlet />
        </main>
      </div>
      <TanStackRouterDevtools position="bottom-right" />
    </RpcProvider>
  );
}

export const Route = createRootRoute({
  component: RootLayout,
});
