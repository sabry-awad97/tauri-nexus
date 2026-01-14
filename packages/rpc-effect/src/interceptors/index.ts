// =============================================================================
// Interceptors Module Exports
// =============================================================================

export {
  type InterceptorOptions,
  type InterceptorHandler,
  createInterceptorFactory,
  createSimpleInterceptor,
  composeInterceptors,
} from "./factory";

export { loggingInterceptor, type LoggingInterceptorOptions } from "./logging";

export { retryInterceptor, type RetryInterceptorOptions } from "./retry";

export { authInterceptor, type AuthInterceptorOptions } from "./auth";

export { timingInterceptor } from "./timing";

export { dedupeInterceptor, type DedupeInterceptorOptions } from "./dedupe";

export { errorHandlerInterceptor } from "./error-handler";
