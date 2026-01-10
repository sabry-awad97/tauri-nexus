import { createFileRoute } from '@tanstack/react-router';
import { useHealth, useUsers } from '../generated';

function StatCard({ 
  icon, 
  label, 
  value, 
  subtext,
  color = 'blue' 
}: { 
  icon: string; 
  label: string; 
  value: string | number; 
  subtext?: string;
  color?: 'blue' | 'green' | 'purple' | 'orange';
}) {
  return (
    <div className={`stat-card ${color}`}>
      <div className="stat-icon">{icon}</div>
      <div className="stat-content">
        <span className="stat-value">{value}</span>
        <span className="stat-label">{label}</span>
        {subtext && <span className="stat-subtext">{subtext}</span>}
      </div>
    </div>
  );
}

function FeatureCard({ 
  icon, 
  title, 
  description, 
  tags 
}: { 
  icon: string; 
  title: string; 
  description: string; 
  tags: string[];
}) {
  return (
    <div className="feature-card">
      <div className="feature-icon">{icon}</div>
      <h3 className="feature-title">{title}</h3>
      <p className="feature-description">{description}</p>
      <div className="feature-tags">
        {tags.map(tag => (
          <span key={tag} className="tag">{tag}</span>
        ))}
      </div>
    </div>
  );
}

function Dashboard() {
  const { data: health } = useHealth();
  const { data: users } = useUsers();

  return (
    <div className="page dashboard">
      <header className="page-header">
        <div>
          <h1 className="page-title">Dashboard</h1>
          <p className="page-subtitle">
            Type-safe RPC framework for Tauri with React hooks
          </p>
        </div>
      </header>

      <section className="stats-grid">
        <StatCard 
          icon="🚀" 
          label="Status" 
          value={health?.status === 'ok' ? 'Online' : 'Offline'}
          subtext={health?.version ? `Version ${health.version}` : undefined}
          color="green"
        />
        <StatCard 
          icon="👥" 
          label="Users" 
          value={users?.length ?? 0}
          subtext="In database"
          color="blue"
        />
        <StatCard 
          icon="📡" 
          label="Subscriptions" 
          value={4}
          subtext="Available streams"
          color="purple"
        />
        <StatCard 
          icon="⚡" 
          label="Procedures" 
          value={9}
          subtext="Query + Mutation"
          color="orange"
        />
      </section>

      <section className="features-section">
        <h2 className="section-title">Features</h2>
        <div className="features-grid">
          <FeatureCard
            icon="🔒"
            title="Type-Safe"
            description="End-to-end type safety from Rust to TypeScript with automatic type inference"
            tags={['TypeScript', 'Rust', 'Serde']}
          />
          <FeatureCard
            icon="🪝"
            title="React Hooks"
            description="Built-in hooks for queries, mutations, and subscriptions with loading states"
            tags={['useQuery', 'useMutation', 'useSubscription']}
          />
          <FeatureCard
            icon="📡"
            title="Real-time Streams"
            description="SSE-style event streaming with async iterators and automatic cleanup"
            tags={['AsyncIterator', 'Events', 'Backpressure']}
          />
          <FeatureCard
            icon="🔌"
            title="Middleware"
            description="Composable middleware for logging, auth, validation, and more"
            tags={['Logging', 'Auth', 'Validation']}
          />
        </div>
      </section>

      <section className="architecture-section">
        <h2 className="section-title">Architecture</h2>
        <div className="architecture-diagram">
          <div className="arch-layer frontend">
            <span className="arch-label">Frontend</span>
            <div className="arch-items">
              <span className="arch-item">React Hooks</span>
              <span className="arch-item">TypeScript Client</span>
              <span className="arch-item">Event Iterator</span>
            </div>
          </div>
          <div className="arch-arrow">↕</div>
          <div className="arch-layer transport">
            <span className="arch-label">Transport</span>
            <div className="arch-items">
              <span className="arch-item">Tauri IPC</span>
              <span className="arch-item">JSON-RPC</span>
            </div>
          </div>
          <div className="arch-arrow">↕</div>
          <div className="arch-layer backend">
            <span className="arch-label">Backend</span>
            <div className="arch-items">
              <span className="arch-item">Router</span>
              <span className="arch-item">Middleware</span>
              <span className="arch-item">Handlers</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

export const Route = createFileRoute('/')({
  component: Dashboard,
});
