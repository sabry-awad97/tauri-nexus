/**
 * RPC exports - use these in your app
 */

import { createClient, createReactClient } from '../lib/tauri-rpc';
import { appRouter, type AppRouter } from './router';

// Vanilla TypeScript client
export const rpc = createClient(appRouter);

// React hooks
export const {
  Provider: RPCProvider,
  useQuery,
  useMutation,
  useClient,
} = createReactClient(appRouter);

// Re-export types
export type { AppRouter };
export { appRouter };
