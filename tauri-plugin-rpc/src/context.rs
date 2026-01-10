//! Context types for RPC handlers

use std::sync::Arc;

/// Context wrapper passed to handlers
#[derive(Clone)]
pub struct Context<T: Clone + Send + Sync + 'static> {
    inner: Arc<T>,
}

impl<T: Clone + Send + Sync + 'static> Context<T> {
    pub fn new(ctx: T) -> Self {
        Self { inner: Arc::new(ctx) }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }
}

impl<T: Clone + Send + Sync + 'static> std::ops::Deref for Context<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Trait for types that can be used as context
pub trait AppContext: Clone + Send + Sync + 'static {}

impl<T: Clone + Send + Sync + 'static> AppContext for T {}

/// Empty context for routers without state
#[derive(Clone, Default)]
pub struct EmptyContext;
