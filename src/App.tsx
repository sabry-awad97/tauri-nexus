import { useState } from 'react';
import { rpc, RPCProvider, useQuery, useMutation } from './rpc';
import './App.css';

// Example 1: Vanilla TypeScript usage
function VanillaExample() {
  const [name, setName] = useState('');
  const [greeting, setGreeting] = useState('');
  const [loading, setLoading] = useState(false);

  async function handleGreet() {
    setLoading(true);
    try {
      // Type-safe! TypeScript knows input is { name: string } and output is string
      const result = await rpc.greet({ name });
      setGreeting(result);
    } catch (error) {
      console.error('Failed:', error);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="example">
      <h2>Vanilla TypeScript</h2>
      <div className="row">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter name..."
        />
        <button onClick={handleGreet} disabled={loading}>
          {loading ? 'Loading...' : 'Greet'}
        </button>
      </div>
      {greeting && <p className="result">{greeting}</p>}
    </div>
  );
}

// Example 2: React hooks usage
function ReactHooksExample() {
  const [name, setName] = useState('React');

  const { data, isLoading, refetch } = useQuery('greet', { name }, { enabled: name.length > 0 });

  return (
    <div className="example">
      <h2>React Hooks</h2>
      <div className="row">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter name..."
        />
        <button onClick={() => refetch()} disabled={isLoading}>
          {isLoading ? 'Loading...' : 'Refetch'}
        </button>
      </div>
      {data && <p className="result">{data}</p>}
    </div>
  );
}

// Example 3: Mutation usage
function MutationExample() {
  const [name, setName] = useState('');

  const mutation = useMutation('greet', {
    onSuccess: (data) => console.log('Success:', data),
  });

  return (
    <div className="example">
      <h2>Mutation Pattern</h2>
      <div className="row">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter name..."
        />
        <button
          onClick={() => mutation.mutate({ name })}
          disabled={mutation.isLoading}
        >
          {mutation.isLoading ? 'Sending...' : 'Send'}
        </button>
      </div>
      {mutation.data && <p className="result">{mutation.data}</p>}
    </div>
  );
}

function AppContent() {
  return (
    <main className="container">
      <h1>Tauri RPC Demo</h1>
      <p>Type-safe communication between React and Rust</p>
      <VanillaExample />
      <ReactHooksExample />
      <MutationExample />
    </main>
  );
}

export default function App() {
  return (
    <RPCProvider>
      <AppContent />
    </RPCProvider>
  );
}
