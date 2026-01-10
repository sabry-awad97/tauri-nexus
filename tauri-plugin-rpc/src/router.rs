//! Router implementation with builder pattern

use crate::{
    handler::{into_boxed_handler, BoxedHandler, Handler},
    middleware::{MiddlewareFn, Next, ProcedureType, Request, Response},
    plugin::DynRouter,
    Context, EmptyContext, RpcError, RpcResult,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Procedure definition
struct Procedure<Ctx: Clone + Send + Sync + 'static> {
    handler: BoxedHandler<Ctx>,
    procedure_type: ProcedureType,
}

/// Router with builder pattern
pub struct Router<Ctx: Clone + Send + Sync + 'static = EmptyContext> {
    context: Option<Ctx>,
    procedures: HashMap<String, Procedure<Ctx>>,
    middleware: Vec<MiddlewareFn<Ctx>>,
    prefix: String,
}

impl Default for Router<EmptyContext> {
    fn default() -> Self {
        Self::new()
    }
}

impl Router<EmptyContext> {
    /// Create a new router without context
    pub fn new() -> Self {
        Self {
            context: None,
            procedures: HashMap::new(),
            middleware: Vec::new(),
            prefix: String::new(),
        }
    }
}

impl<Ctx: Clone + Send + Sync + 'static> Router<Ctx> {
    /// Set the context for this router
    pub fn context<NewCtx: Clone + Send + Sync + 'static>(self, ctx: NewCtx) -> Router<NewCtx> {
        Router {
            context: Some(ctx),
            procedures: HashMap::new(),
            middleware: Vec::new(),
            prefix: self.prefix,
        }
    }

    /// Add middleware to the router
    pub fn middleware<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(f(ctx, req, next))
        }));
        self
    }

    /// Add a query procedure (read-only operation)
    pub fn query<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure {
                handler: into_boxed_handler(handler),
                procedure_type: ProcedureType::Query,
            },
        );
        self
    }

    /// Add a mutation procedure (write operation)
    pub fn mutation<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure {
                handler: into_boxed_handler(handler),
                procedure_type: ProcedureType::Mutation,
            },
        );
        self
    }

    /// Merge another router under a namespace
    pub fn merge<N: Into<String>>(mut self, namespace: N, other: Router<Ctx>) -> Self {
        let namespace = namespace.into();
        for (path, procedure) in other.procedures {
            let full_path = if namespace.is_empty() {
                path
            } else {
                format!("{}.{}", namespace, path)
            };
            self.procedures.insert(full_path, procedure);
        }
        // Merge middleware
        self.middleware.extend(other.middleware);
        self
    }

    fn make_path(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", self.prefix, name)
        }
    }

    /// Get the context
    pub fn get_context(&self) -> Option<&Ctx> {
        self.context.as_ref()
    }

    /// List all procedure paths
    pub fn procedures(&self) -> Vec<String> {
        self.procedures.keys().cloned().collect()
    }

    /// Call a procedure by path
    pub async fn call(&self, path: &str, input: serde_json::Value) -> RpcResult<serde_json::Value> {
        let procedure = self
            .procedures
            .get(path)
            .ok_or_else(|| RpcError::procedure_not_found(path))?;

        let ctx = Context::new(
            self.context
                .clone()
                .ok_or_else(|| RpcError::internal("Router context not set"))?,
        );

        let request = Request {
            path: path.to_string(),
            procedure_type: procedure.procedure_type.clone(),
            input: input.clone(),
        };

        // Build middleware chain
        let handler = procedure.handler.clone();
        let final_handler: Next<Ctx> = Arc::new(move |ctx, req| {
            let handler = handler.clone();
            Box::pin(async move { (handler)(ctx, req.input).await })
        });

        // Apply middleware in reverse order
        let chain = self.middleware.iter().rev().fold(final_handler, |next, mw| {
            let mw = mw.clone();
            Arc::new(move |ctx, req| {
                let mw = mw.clone();
                let next = next.clone();
                Box::pin(async move { (mw)(ctx, req, next).await })
            })
        });

        chain(ctx, request).await
    }
}

/// Builder for creating routers with a specific namespace
pub struct RouterBuilder<Ctx: Clone + Send + Sync + 'static> {
    router: Router<Ctx>,
}

impl<Ctx: Clone + Send + Sync + 'static> RouterBuilder<Ctx> {
    pub fn new(prefix: &str) -> Self {
        Self {
            router: Router {
                context: None,
                procedures: HashMap::new(),
                middleware: Vec::new(),
                prefix: prefix.to_string(),
            },
        }
    }

    pub fn query<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        self.router = self.router.query(name, handler);
        self
    }

    pub fn mutation<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        self.router = self.router.mutation(name, handler);
        self
    }

    pub fn build(self) -> Router<Ctx> {
        self.router
    }
}


// Implement DynRouter for Router
impl<Ctx: Clone + Send + Sync + 'static> DynRouter for Router<Ctx> {
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, RpcError>> + Send + 'a>> {
        Box::pin(async move { Router::call(self, path, input).await })
    }

    fn procedures(&self) -> Vec<String> {
        Router::procedures(self)
    }
}
