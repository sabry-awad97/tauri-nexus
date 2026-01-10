/**
 * Router definition - defines all Tauri commands with types
 */

import { router, procedure } from '../lib/tauri-rpc';

// Define your Tauri commands here
export const appRouter = router({
  // Greet command - matches the Rust greet function
  greet: procedure()
    .command('greet')
    .input<{ name: string }>()
    .output<string>()
    .query(),
});

export type AppRouter = typeof appRouter;
