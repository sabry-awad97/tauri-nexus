// =============================================================================
// Subscription Module Exports
// =============================================================================

export {
  type SubscriptionEventType,
  type SubscriptionEvent,
  type SubscriptionError,
  type SubscriptionState,
  type ReconnectConfig,
  type QueueItem,
  SHUTDOWN_SENTINEL,
  defaultReconnectConfig,
} from "./types";

export {
  createSubscriptionState,
  createSubscriptionStateRef,
  createEventQueue,
  markCompleted,
  updateLastEventId,
  incrementConsumers,
  decrementConsumers,
  resetForReconnect,
  incrementReconnectAttempts,
  resetReconnectAttempts,
} from "./state";

export { offerEvent, sendShutdownSentinels, takeFromQueue } from "./queue";

export {
  calculateReconnectDelay,
  shouldReconnect,
  prepareReconnect,
  waitForReconnect,
  maxReconnectsExceededError,
} from "./reconnect";

export {
  processDataEvent,
  processErrorEvent,
  generateSubscriptionId,
  extractSubscriptionError,
} from "./events";

// Stream-based API (Effect-idiomatic)
export {
  type SubscriptionStreamConfig,
  type AsyncIteratorConfig,
  createSubscriptionStream,
  createManagedSubscriptionStream,
  scopedConnection,
  collectStream,
  runStreamWithCallbacks,
  runStreamInterruptible,
  createAsyncIterator,
} from "./stream";
