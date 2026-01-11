import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { orpc } from "../rpc/contract";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

function StatCard({
  icon,
  label,
  value,
  subtext,
  variant = "default",
}: {
  icon: string;
  label: string;
  value: string | number;
  subtext?: string;
  variant?: "default" | "success" | "info" | "warning";
}) {
  const borderColors = {
    default: "border-l-primary",
    success: "border-l-green-500",
    info: "border-l-blue-500",
    warning: "border-l-orange-500",
  };

  return (
    <Card className={`border-l-4 ${borderColors[variant]}`}>
      <CardContent className="flex items-center gap-4 p-5">
        <span className="text-3xl">{icon}</span>
        <div className="flex flex-col">
          <span className="text-2xl font-bold">{value}</span>
          <span className="text-sm text-muted-foreground">{label}</span>
          {subtext && (
            <span className="text-xs text-muted-foreground/70">{subtext}</span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function FeatureCard({
  icon,
  title,
  description,
  tags,
}: {
  icon: string;
  title: string;
  description: string;
  tags: string[];
}) {
  return (
    <Card className="transition-all hover:border-muted-foreground/30 hover:-translate-y-0.5">
      <CardHeader className="pb-3">
        <span className="text-3xl mb-2">{icon}</span>
        <CardTitle className="text-base">{title}</CardTitle>
        <CardDescription className="text-sm leading-relaxed">
          {description}
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="flex flex-wrap gap-1.5">
          {tags.map((tag) => (
            <Badge
              key={tag}
              variant="secondary"
              className="text-xs font-normal"
            >
              {tag}
            </Badge>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function Dashboard() {
  const { data: health } = useQuery(orpc.health.queryOptions());
  const { data: users } = useQuery(orpc.user.list.queryOptions());

  return (
    <div className="p-8 max-w-6xl mx-auto space-y-10">
      <header>
        <h1 className="text-3xl font-bold mb-2">Dashboard</h1>
        <p className="text-muted-foreground">
          Type-safe RPC framework for Tauri with React hooks
        </p>
      </header>

      <section className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon="🚀"
          label="Status"
          value={health?.status === "ok" ? "Online" : "Offline"}
          subtext={health?.version ? `Version ${health.version}` : undefined}
          variant="success"
        />
        <StatCard
          icon="👥"
          label="Users"
          value={users?.length ?? 0}
          subtext="In database"
          variant="info"
        />
        <StatCard
          icon="📡"
          label="Subscriptions"
          value={4}
          subtext="Available streams"
          variant="default"
        />
        <StatCard
          icon="⚡"
          label="Procedures"
          value={9}
          subtext="Query + Mutation"
          variant="warning"
        />
      </section>

      <section>
        <h2 className="text-lg font-semibold mb-4">Features</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <FeatureCard
            icon="🔒"
            title="Type-Safe"
            description="End-to-end type safety from Rust to TypeScript with automatic type inference"
            tags={["TypeScript", "Rust", "Serde"]}
          />
          <FeatureCard
            icon="🪝"
            title="React Hooks"
            description="Built-in hooks for queries, mutations, and subscriptions with loading states"
            tags={["useQuery", "useMutation", "useSubscription"]}
          />
          <FeatureCard
            icon="📡"
            title="Real-time Streams"
            description="SSE-style event streaming with async iterators and automatic cleanup"
            tags={["AsyncIterator", "Events", "Backpressure"]}
          />
          <FeatureCard
            icon="🔌"
            title="Middleware"
            description="Composable middleware for logging, auth, validation, and more"
            tags={["Logging", "Auth", "Validation"]}
          />
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold mb-4">Architecture</h2>
        <Card>
          <CardContent className="p-8 flex flex-col items-center gap-4">
            <div className="w-full max-w-md p-4 rounded-lg bg-indigo-500/10 border border-indigo-500/30 text-center">
              <span className="text-xs font-semibold uppercase tracking-wide text-indigo-400 block mb-2">
                Frontend
              </span>
              <div className="flex justify-center gap-2 flex-wrap">
                <Badge variant="secondary">React Hooks</Badge>
                <Badge variant="secondary">TypeScript Client</Badge>
                <Badge variant="secondary">Event Iterator</Badge>
              </div>
            </div>
            <span className="text-xl text-muted-foreground">↕</span>
            <div className="w-full max-w-md p-4 rounded-lg bg-violet-500/10 border border-violet-500/30 text-center">
              <span className="text-xs font-semibold uppercase tracking-wide text-violet-400 block mb-2">
                Transport
              </span>
              <div className="flex justify-center gap-2 flex-wrap">
                <Badge variant="secondary">Tauri IPC</Badge>
                <Badge variant="secondary">JSON-RPC</Badge>
              </div>
            </div>
            <span className="text-xl text-muted-foreground">↕</span>
            <div className="w-full max-w-md p-4 rounded-lg bg-green-500/10 border border-green-500/30 text-center">
              <span className="text-xs font-semibold uppercase tracking-wide text-green-400 block mb-2">
                Backend
              </span>
              <div className="flex justify-center gap-2 flex-wrap">
                <Badge variant="secondary">Router</Badge>
                <Badge variant="secondary">Middleware</Badge>
                <Badge variant="secondary">Handlers</Badge>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}

export const Route = createFileRoute("/")({
  component: Dashboard,
});
