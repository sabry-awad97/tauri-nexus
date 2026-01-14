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
  // Atomic operations using Ref.modify
  incrementAndGetConsumers,
  decrementAndGetConsumers,
  incrementAndGetReconnectAttempts,
  markCompletedOnce,
  updateAndGetLastEventId,
  getState,
} from "./state";

export { offerEvent, sendShutdownSentinels, takeFromQueue } from "./queue";

export {
  // Schedule-based reconnection
  createReconnectSchedule,
  withReconnection,
  // Legacy functions (still supported)
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
  // Resource management
  withSubscription,
  // PubSub for multi-consumer
  type BroadcastSubscription,
  createBroadcastSubscription,
  createScopedBroadcastSubscription,
  // Stream from async iterable
  createEventSourceStream,
} from "./stream";
