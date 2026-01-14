// =============================================================================
// RPC Transport Service
// =============================================================================

import { Context, Layer } from "effect";
import type { RpcTransport } from "../core/types";

/**
 * Transport service - no default implementation.
 * Must be provided by the user (e.g., TauriTransport, FetchTransport).
 *
 * @example
 * ```ts
 * Effect.provide(program, RpcTransportService.layer(myTransport))
 * ```
 */
export class RpcTransportService extends Context.Tag("RpcTransportService")<
  RpcTransportService,
  RpcTransport
>() {
  /** Create a layer with the given transport */
  static layer(transport: RpcTransport) {
    return Layer.succeed(RpcTransportService, transport);
  }
}
