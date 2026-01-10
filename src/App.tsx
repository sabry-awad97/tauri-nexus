import { useState } from 'react';
import {
  RpcProvider,
  useGreet,
  useListUsers,
  useCreateUser,
  useDeleteUser,
} from './generated/hooks';
import { rpc } from './generated';
import type { User } from './generated';
import './App.css';

// Example 1: Vanilla TypeScript usage
function VanillaExample() {
  const [name, setName] = useState('');
  const [greeting, setGreeting] = useState('');
  const [loading, setLoading] = useState(false);

  async function handleGreet() {
    setLoading(true);
    try {
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
  const { data, isLoading, refetch } = useGreet({ name }, { enabled: name.length > 0 });

  return (
    <div className="example">
      <h2>React Hooks (useGreet)</h2>
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

// Example 3: User list with CRUD
function UserListExample() {
  const { data: usersData, isLoading, refetch } = useListUsers({});
  const createUser = useCreateUser({
    onSuccess: () => refetch(),
  });
  const deleteUser = useDeleteUser({
    onSuccess: () => refetch(),
  });

  const [newName, setNewName] = useState('');
  const [newEmail, setNewEmail] = useState('');

  const handleCreate = () => {
    if (newName && newEmail) {
      createUser.mutate({ input: { name: newName, email: newEmail } });
      setNewName('');
      setNewEmail('');
    }
  };

  return (
    <div className="example">
      <h2>User Management (Generated Types)</h2>
      
      <div className="row" style={{ marginBottom: '1rem' }}>
        <input
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="Name"
        />
        <input
          value={newEmail}
          onChange={(e) => setNewEmail(e.target.value)}
          placeholder="Email"
        />
        <button onClick={handleCreate} disabled={createUser.isLoading}>
          {createUser.isLoading ? 'Creating...' : 'Add User'}
        </button>
      </div>

      {isLoading ? (
        <p>Loading users...</p>
      ) : (
        <div className="user-list">
          {usersData?.data.map((user: User) => (
            <div key={user.id} className="user-item">
              <span>{user.name} ({user.email})</span>
              <button
                onClick={() => deleteUser.mutate({ id: user.id })}
                disabled={deleteUser.isLoading}
                style={{ marginLeft: '0.5rem', background: '#dc2626' }}
              >
                Delete
              </button>
            </div>
          ))}
          <p style={{ fontSize: '0.8rem', color: '#888' }}>
            Total: {usersData?.total} | Page: {usersData?.page}/{usersData?.totalPages}
          </p>
        </div>
      )}
    </div>
  );
}

function AppContent() {
  return (
    <main className="container">
      <h1>Tauri RPC Plugin Demo</h1>
      <p>Type-safe communication with auto-generated types</p>
      <VanillaExample />
      <ReactHooksExample />
      <UserListExample />
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
