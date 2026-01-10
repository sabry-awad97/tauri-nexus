// =============================================================================
// RPC Contract - Type definitions for Tauri RPC
// =============================================================================
// 
// These types mirror the Rust types in src-tauri/src/rpc/types.rs
// In the future, this will be auto-generated from Rust proc macros.

import { createClientWithSubscriptions, type ContractRouter } from '../lib/rpc';

// =============================================================================
// Domain Types
// =============================================================================

export interface User {
  id: number;
  name: string;
  email: string;
  createdAt: string;
}

export interface CreateUserInput {
  name: string;
  email: string;
}

export interface UpdateUserInput {
  id: number;
  name?: string;
  email?: string;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export interface SuccessResponse {
  success: boolean;
  message?: string;
}

// =============================================================================
// Subscription Types
// =============================================================================

export interface CounterInput {
  start?: number;
  maxCount?: number;
  intervalMs?: number;
}

export interface CounterEvent {
  count: number;
  timestamp: string;
}

export interface ChatRoomInput {
  roomId: string;
}

export interface ChatMessage {
  id: string;
  roomId: string;
  userId: string;
  text: string;
  timestamp: string;
}

export interface SendMessageInput {
  roomId: string;
  text: string;
}

export interface StockInput {
  symbols: string[];
}

export interface StockPrice {
  symbol: string;
  price: number;
  change: number;
  changePercent: number;
  timestamp: string;
}

// =============================================================================
// Contract Definition
// =============================================================================

export interface AppContract extends ContractRouter {
  // Root procedures
  health: { type: 'query'; input: void; output: HealthResponse };
  greet: { type: 'query'; input: { name: string }; output: string };
  
  // User namespace
  user: {
    get: { type: 'query'; input: { id: number }; output: User };
    list: { type: 'query'; input: void; output: User[] };
    create: { type: 'mutation'; input: CreateUserInput; output: User };
    update: { type: 'mutation'; input: UpdateUserInput; output: User };
    delete: { type: 'mutation'; input: { id: number }; output: SuccessResponse };
  };
  
  // Stream namespace (subscriptions)
  stream: {
    // Counter - emits incrementing numbers
    counter: { type: 'subscription'; input: CounterInput; output: CounterEvent };
    // Stocks - simulated real-time stock prices
    stocks: { type: 'subscription'; input: StockInput; output: StockPrice };
    // Chat - chat room messages
    chat: { type: 'subscription'; input: ChatRoomInput; output: ChatMessage };
    // Time - current time every second
    time: { type: 'subscription'; input: void; output: string };
  };
}

// =============================================================================
// Subscription Paths
// =============================================================================

const SUBSCRIPTION_PATHS = [
  'stream.counter',
  'stream.stocks',
  'stream.chat',
  'stream.time',
] as const;

// =============================================================================
// Typed Client Export
// =============================================================================

export const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: [...SUBSCRIPTION_PATHS],
});

// Namespace exports for convenience
export const user = rpc.user;
export const stream = rpc.stream;
