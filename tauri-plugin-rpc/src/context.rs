//! Context types for dependency injection

use std::sync::Arc;

/// Context wrapper providing access to application state
/// 
/// The context is cloned for each request, so use `Arc` for shared state.
#[derive(Clone)]
pub struct Context<T: Clone + Send + Sync + 'static> {
    inner: Arc<T>,
}

impl<T: Clone + Send + Sync + 'static> Context<T> {
    /// Create a new context wrapping the given value
    pub fn new(ctx: T) -> Self {
        Self { inner: Arc::new(ctx) }
    }

    /// Get a reference to the inner context
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get the Arc for sharing
    pub fn arc(&self) -> Arc<T> {
        self.inner.clone()
    }
}

impl<T: Clone + Send + Sync + 'static> std::ops::Deref for Context<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Clone + Send + Sync + 'static + Default> Default for Context<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Empty context for routers that don't need state
#[derive(Clone, Default, Debug)]
pub struct EmptyContext;
