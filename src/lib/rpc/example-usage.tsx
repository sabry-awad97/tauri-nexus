// =============================================================================
// Example: How to Use the Type-Safe RPC Library
// =============================================================================
// 
// This file demonstrates how to define your contract types and create
// a fully type-safe client. You only need to define types - everything
// else is inferred automatically!

import { 
  createTypedClient,
  createClientWithSubscriptions,
  query, 
  mutation, 
  subscription,
  subscribe,
  type ContractRouter,
} from './index';
import { useQuery, useMutation, useSubscription } from './hooks';

// =============================================================================
// Step 1: Define Your Types (mirror your Rust types)
// =============================================================================

// Domain types
interface User {
  id: number;
  name: string;
  email: string;
  createdAt: string;
}

interface CreateUserInput {
  name: string;
  email: string;
}

interface UpdateUserInput {
  id: number;
  name?: string;
  email?: string;
}

interface ChatMessage {
  id: string;
  userId: string;
  text: string;
  timestamp: string;
}

interface ChatInput {
  roomId: string;
}

interface SendMessageInput {
  roomId: string;
  text: string;
}

interface StockPrice {
  symbol: string;
  price: number;
  change: number;
  timestamp: string;
}

interface StockInput {
  symbols: string[];
}

// =============================================================================
// Step 2: Define Your Contract (matches your Rust router structure)
// =============================================================================

// Define the contract type - this is all you need!
interface AppContract extends ContractRouter {
  // Health check
  health: { type: 'query'; input: void; output: { status: string; version: string } };
  
  // User procedures
  user: {
    get: { type: 'query'; input: { id: number }; output: User };
    list: { type: 'query'; input: void; output: User[] };
    create: { type: 'mutation'; input: CreateUserInput; output: User };
    update: { type: 'mutation'; input: UpdateUserInput; output: User };
    delete: { type: 'mutation'; input: { id: number }; output: { success: boolean } };
  };
  
  // Chat procedures (with subscriptions!)
  chat: {
    messages: { type: 'subscription'; input: ChatInput; output: ChatMessage };
    send: { type: 'mutation'; input: SendMessageInput; output: ChatMessage };
  };
  
  // Stock streaming
  stocks: {
    live: { type: 'subscription'; input: StockInput; output: StockPrice };
    latest: { type: 'query'; input: { symbol: string }; output: StockPrice };
  };
}

// =============================================================================
// Step 3: Create Your Client (fully type-safe!)
// =============================================================================

// Option A: Simple client (subscriptions need manual handling)
export const rpc = createTypedClient<AppContract>();

// Option B: Client with subscription paths registered (recommended for subscriptions)
export const rpcWithSubs = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: ['chat.messages', 'stocks.live'],
});

// =============================================================================
// Step 4: Use It! (with full autocomplete and type checking)
// =============================================================================

// Queries - input and output are fully typed
async function queryExamples() {
  // ✅ Type-safe: input is { id: number }, output is User
  const user = await rpc.user.get({ id: 1 });
  console.log(user.name); // ✅ Autocomplete works!
  
  // ✅ Type-safe: no input required, output is User[]
  const users = await rpc.user.list();
  users.forEach((u: User) => console.log(u.email));
  
  // ✅ Type-safe: input is CreateUserInput, output is User
  const newUser = await rpc.user.create({ 
    name: 'Alice', 
    email: 'alice@example.com' 
  });
  console.log('Created:', newUser.id);
  
  // ❌ TypeScript Error: Property 'invalid' does not exist
  // const invalid = await rpc.user.invalid();
  
  // ❌ TypeScript Error: Argument of type 'string' is not assignable
  // const wrongType = await rpc.user.get({ id: 'not-a-number' });
}

// Subscriptions - using the subscribe function directly
async function subscriptionExamples() {
  // ✅ Type-safe: input is StockInput, yields StockPrice
  const stockStream = await subscribe<StockPrice>('stocks.live', { symbols: ['AAPL', 'GOOGL'] });
  
  for await (const price of stockStream) {
    console.log(`${price.symbol}: ${price.price}`); // ✅ Autocomplete!
  }
  
  // ✅ Type-safe: input is ChatInput, yields ChatMessage
  const chatStream = await subscribe<ChatMessage>('chat.messages', { roomId: 'general' });
  
  for await (const message of chatStream) {
    console.log(`${message.userId}: ${message.text}`);
  }
}

// =============================================================================
// Step 5: React Hooks (also type-safe!)
// =============================================================================

// In a React component:
export function UserProfile({ userId }: { userId: number }) {
  // ✅ Type-safe: data is User | undefined
  const { data: user, isLoading, error } = useQuery(
    () => rpc.user.get({ id: userId }),
    [userId]
  );
  
  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;
  
  return (
    <div>
      <h1>{user?.name}</h1>
      <p>{user?.email}</p>
    </div>
  );
}

export function CreateUserForm() {
  // ✅ Type-safe: mutate expects CreateUserInput, data is User
  const { mutate, isLoading, data } = useMutation<CreateUserInput, User>(
    (input) => rpc.user.create(input)
  );
  
  return (
    <form onSubmit={(e) => {
      e.preventDefault();
      mutate({ name: 'Bob', email: 'bob@example.com' });
    }}>
      <button disabled={isLoading}>Create User</button>
      {data && <p>Created: {data.name}</p>}
    </form>
  );
}

export function StockTicker() {
  // ✅ Type-safe: data is StockPrice[], latestEvent is StockPrice
  const { data: prices, latestEvent, isConnected } = useSubscription<StockPrice>(
    async () => subscribe<StockPrice>('stocks.live', { symbols: ['AAPL'] }),
    [],
    {
      onEvent: (price) => console.log('New price:', price.price),
    }
  );
  
  return (
    <div>
      <span>{isConnected ? '🟢' : '🔴'}</span>
      <span>AAPL: ${latestEvent?.price ?? 'Loading...'}</span>
      <span>Total updates: {prices.length}</span>
    </div>
  );
}

export function ChatRoom({ roomId }: { roomId: string }) {
  // ✅ Type-safe subscription
  const { data: messages, isConnected } = useSubscription<ChatMessage>(
    async () => subscribe<ChatMessage>('chat.messages', { roomId }),
    [roomId],
    { autoReconnect: true }
  );
  
  // ✅ Type-safe mutation
  const sendMessage = useMutation<SendMessageInput, ChatMessage>(
    (input) => rpc.chat.send(input)
  );
  
  return (
    <div>
      <div>{isConnected ? 'Connected' : 'Reconnecting...'}</div>
      {messages.map((m) => (
        <div key={m.id}>{m.text}</div>
      ))}
      <input onKeyDown={(e) => {
        if (e.key === 'Enter') {
          sendMessage.mutate({ 
            roomId, 
            text: e.currentTarget.value 
          });
        }
      }} />
    </div>
  );
}

// =============================================================================
// Alternative: Using Contract Builder Helpers
// =============================================================================

// If you prefer a more explicit contract definition:
const contract = {
  health: query<void, { status: string; version: string }>(),
  user: {
    get: query<{ id: number }, User>(),
    list: query<void, User[]>(),
    create: mutation<CreateUserInput, User>(),
    update: mutation<UpdateUserInput, User>(),
    delete: mutation<{ id: number }, { success: boolean }>(),
  },
  chat: {
    messages: subscription<ChatInput, ChatMessage>(),
    send: mutation<SendMessageInput, ChatMessage>(),
  },
  stocks: {
    live: subscription<StockInput, StockPrice>(),
    latest: query<{ symbol: string }, StockPrice>(),
  },
} as const;

// Create client from contract
type Contract = typeof contract;
export const rpc2 = createTypedClient<Contract>();

// Same type-safe usage!
async function example2() {
  const user = await rpc2.user.get({ id: 1 });
  console.log(user.name);
}

// Export examples for testing
export { queryExamples, subscriptionExamples, example2 };
