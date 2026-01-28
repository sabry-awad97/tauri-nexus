//! Internal types for the router module
//!
//! This module contains internal type definitions used across the router
//! implementation. These types are not part of the public API.

use crate::{
    handler::BoxedHandler,
    middleware::{Next, ProcedureType},
    subscription::BoxedSubscriptionHandler,
};

/// Procedure definition (internal)
///
/// Represents either a query/mutation handler or a subscription handler.
pub(crate) enum Procedure<Ctx: Clone + Send + Sync + 'static> {
    /// Query or Mutation procedure
    Handler {
        handler: BoxedHandler<Ctx>,
        procedure_type: ProcedureType,
    },
    /// Subscription procedure
    Subscription {
        handler: BoxedSubscriptionHandler<Ctx>,
    },
}

/// Pre-compiled middleware chain for a procedure (internal)
///
/// Contains the fully composed middleware chain ready for execution.
pub(crate) struct CompiledChain<Ctx: Clone + Send + Sync + 'static> {
    /// The final handler wrapped with all middleware
    pub(crate) chain: Next<Ctx>,
    /// Procedure type metadata
    pub(crate) procedure_type: ProcedureType,
}
