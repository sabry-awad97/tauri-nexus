import { useState } from 'react';
import {
  RpcProvider,
  useHealth,
  useGreet,
  useUsers,
  useCreateUser,
  useDeleteUser,
  rpc,
  configure,
  type User,
} from './generated';
import './App.css';

// Configure RPC client with logging
configure({
  onError: (path, error) => {
    console.error(`[RPC Error] ${path}:`, error.message);
  },
});

// =============================================================================
// Components
// =============================================================================

function HealthStatus() {
  const { data, isLoading, error } = useHealth({ refetchInterval: 30000 });

  if (isLoading) return <span className="status loading">●</span>;
  if (error) return <span className="status error">●</span>;
  return <span className="status ok">● v{data?.version}</span>;
}

function GreetDemo() {
  const [name, setName] = useState('');
  const [greeting, setGreeting] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  async function handleGreet() {
    if (!name.trim()) return;
    setLoading(true);
    setError('');
    try {
      const result = await rpc.greet({ name });
      setGreeting(result);
    } catch (err: any) {
      setError(err.message || 'Failed to greet');
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="card">
      <h2>👋 Greet</h2>
      <p className="description">Simple query example with validation</p>
      
      <div className="input-group">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleGreet()}
          placeholder="Enter your name..."
          className="input"
        />
        <button onClick={handleGreet} disabled={loading || !name.trim()} className="btn primary">
          {loading ? 'Loading...' : 'Greet'}
        </button>
      </div>
      
      {greeting && <div className="result success">{greeting}</div>}
      {error && <div className="result error">{error}</div>}
    </section>
  );
}

function GreetWithHook() {
  const [name, setName] = useState('World');
  const { data, isLoading, error, refetch } = useGreet({ name }, { enabled: name.length > 0 });

  return (
    <section className="card">
      <h2>🪝 useGreet Hook</h2>
      <p className="description">Reactive query with automatic refetch</p>
      
      <div className="input-group">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Type to greet..."
          className="input"
        />
        <button onClick={() => refetch()} disabled={isLoading} className="btn">
          Refetch
        </button>
      </div>
      
      {isLoading && <div className="result loading">Loading...</div>}
      {error && <div className="result error">{error.message}</div>}
      {data && !isLoading && <div className="result success">{data}</div>}
    </section>
  );
}

function UserList() {
  const { data: users, isLoading, error, refetch } = useUsers();
  const createUser = useCreateUser({ onSuccess: () => refetch() });
  const deleteUser = useDeleteUser({ onSuccess: () => refetch() });

  const [name, setName] = useState('');
  const [email, setEmail] = useState('');

  const handleCreate = () => {
    if (!name.trim() || !email.trim()) return;
    createUser.mutate({ name, email });
    setName('');
    setEmail('');
  };

  return (
    <section className="card">
      <h2>👥 User Management</h2>
      <p className="description">Full CRUD with mutations and optimistic updates</p>
      
      <div className="form-row">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Name"
          className="input"
        />
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="Email"
          className="input"
        />
        <button 
          onClick={handleCreate} 
          disabled={createUser.isLoading || !name.trim() || !email.trim()}
          className="btn primary"
        >
          {createUser.isLoading ? 'Adding...' : 'Add User'}
        </button>
      </div>

      {createUser.error && (
        <div className="result error">{createUser.error.message}</div>
      )}

      <div className="user-list">
        {isLoading && <div className="loading-state">Loading users...</div>}
        {error && <div className="error-state">{error.message}</div>}
        
        {users?.map((user: User) => (
          <div key={user.id} className="user-item">
            <div className="user-info">
              <span className="user-name">{user.name}</span>
              <span className="user-email">{user.email}</span>
            </div>
            <button
              onClick={() => deleteUser.mutate({ id: user.id })}
              disabled={deleteUser.isLoading}
              className="btn danger small"
            >
              Delete
            </button>
          </div>
        ))}
        
        {users && users.length === 0 && (
          <div className="empty-state">No users yet. Add one above!</div>
        )}
      </div>

      <div className="footer">
        Total: {users?.length ?? 0} users
      </div>
    </section>
  );
}

// =============================================================================
// App
// =============================================================================

function AppContent() {
  return (
    <main className="container">
      <header className="header">
        <h1>Tauri RPC</h1>
        <HealthStatus />
      </header>
      
      <p className="subtitle">
        Type-safe ORPC-style router with context, middleware, and React hooks
      </p>

      <div className="grid">
        <GreetDemo />
        <GreetWithHook />
      </div>
      
      <UserList />
    </main>
  );
}

export default function App() {
  return (
    <RpcProvider>
      <AppContent />
    </RpcProvider>
  );
}
