//! DynRouter trait implementations
//!
//! This module contains the DynRouter trait implementations for Router and CompiledRouter.

use super::core::{CompiledRouter, Router};
use crate::{
    RpcResult,
    plugin::DynRouter,
    subscription::{Event, SubscriptionContext},
};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

// =============================================================================
// DynRouter for CompiledRouter
// =============================================================================

impl<Ctx: Clone + Send + Sync + 'static> DynRouter for CompiledRouter<Ctx> {
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send + 'a>> {
        Box::pin(async move { CompiledRouter::call(self, path, input).await })
    }

    fn procedures(&self) -> Vec<String> {
        CompiledRouter::procedures(self)
    }

    fn is_subscription(&self, path: &str) -> bool {
        CompiledRouter::is_subscription(self, path)
    }

    fn subscribe<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
        ctx: SubscriptionContext,
    ) -> Pin<
        Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send + 'a>,
    > {
        Box::pin(async move { CompiledRouter::subscribe(self, path, input, ctx).await })
    }
}

// =============================================================================
// DynRouter for Router
// =============================================================================

impl<Ctx: Clone + Send + Sync + 'static> DynRouter for Router<Ctx> {
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send + 'a>> {
        Box::pin(async move { Router::call(self, path, input).await })
    }

    fn procedures(&self) -> Vec<String> {
        Router::procedures(self)
    }

    fn is_subscription(&self, path: &str) -> bool {
        Router::is_subscription(self, path)
    }

    fn subscribe<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
        ctx: SubscriptionContext,
    ) -> Pin<
        Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send + 'a>,
    > {
        Box::pin(async move { Router::subscribe(self, path, input, ctx).await })
    }
}
