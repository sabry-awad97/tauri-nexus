// =============================================================================
// Serializable Module Exports
// =============================================================================

export type { RpcError, RpcErrorCode, RpcErrorShape } from "./types";

export {
  toRpcError,
  fromRpcError,
  isRpcError,
  hasErrorCode,
  createRpcError,
  isRateLimitError,
  getRateLimitRetryAfter,
} from "./conversion";

export {
  isRpcErrorShape,
  parseJsonError,
  createCallErrorFromShape,
  type ErrorParserOptions,
  parseToEffectError,
  fromTransportError,
  parseEffectError,
  parseError,
} from "./parsing";
