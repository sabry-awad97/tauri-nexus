//! RPC Handlers
//!
//! Define your handlers here and register them in create_router().

use super::*;
use async_stream::stream;
use std::pin::pin;
use tauri_plugin_rpc::middleware::{Next, Request, Response};
use tauri_plugin_rpc::subscription::{event_channel, Event, EventStream, SubscriptionContext};
use tokio::time::{interval, Duration};
use tokio_stream::StreamExt;

// =============================================================================
// Middleware
// =============================================================================

/// Logging middleware - logs all RPC calls
pub async fn logging(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    let start = std::time::Instant::now();
    println!("→ [{}] {}", req.procedure_type, req.path);

    let result = next(ctx, req.clone()).await;
    let duration = start.elapsed();

    match &result {
        Ok(_) => println!("← [{}] {} ({:?})", req.procedure_type, req.path, duration),
        Err(e) => println!(
            "✗ [{}] {} - {} ({:?})",
            req.procedure_type, req.path, e.code, duration
        ),
    }

    result
}

// =============================================================================
// Router
// =============================================================================

/// Create the application router
pub fn create_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .middleware(logging)
        // Root procedures
        .query("health", health_handler)
        .query("greet", greet_handler)
        // User procedures
        .merge("user", user_router())
        // Subscription examples
        .merge("stream", stream_router())
}

/// User sub-router
fn user_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .query("get", get_user)
        .query("list", list_users)
        .mutation("create", create_user)
        .mutation("update", update_user)
        .mutation("delete", delete_user)
}

/// Stream/Subscription sub-router
fn stream_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        // Simple counter that emits numbers
        .subscription("counter", counter_subscription)
        // Simulated stock prices
        .subscription("stocks", stock_subscription)
        // Chat room messages
        .subscription("chat", chat_subscription)
        // Time ticker
        .subscription("time", time_subscription)
}

// =============================================================================
// Root Handlers
// =============================================================================

async fn health_handler(_ctx: Context<AppContext>, _: ()) -> RpcResult<HealthResponse> {
    Ok(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

async fn greet_handler(_ctx: Context<AppContext>, input: GreetInput) -> RpcResult<String> {
    if input.name.is_empty() {
        return Err(RpcError::validation("Name cannot be empty"));
    }
    Ok(format!("Hello, {}! 👋", input.name))
}

// =============================================================================
// User Handlers
// =============================================================================

async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    ctx.db
        .get_user(input.id)
        .await
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

async fn list_users(ctx: Context<AppContext>, _: ()) -> RpcResult<Vec<User>> {
    Ok(ctx.db.list_users().await)
}

async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    if input.name.trim().is_empty() {
        return Err(RpcError::validation("Name is required"));
    }
    if !input.email.contains('@') {
        return Err(RpcError::validation("Invalid email format"));
    }

    ctx.db
        .create_user(&input.name, &input.email)
        .await
}

async fn update_user(ctx: Context<AppContext>, input: UpdateUserInput) -> RpcResult<User> {
    if let Some(ref email) = input.email {
        if !email.contains('@') {
            return Err(RpcError::validation("Invalid email format"));
        }
    }

    ctx.db
        .update_user(input.id, input.name.as_deref(), input.email.as_deref())
        .await
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

async fn delete_user(
    ctx: Context<AppContext>,
    input: DeleteUserInput,
) -> RpcResult<SuccessResponse> {
    if ctx.db.delete_user(input.id).await {
        Ok(SuccessResponse::ok(format!("User {} deleted", input.id)))
    } else {
        Err(RpcError::not_found(format!("User {} not found", input.id)))
    }
}

// =============================================================================
// Subscription Handlers using async_stream::stream!
// =============================================================================

/// Counter subscription - emits incrementing numbers using async_stream
///
/// Example usage:
/// ```typescript
/// const stream = await rpc.stream.counter({ start: 0, maxCount: 10, intervalMs: 500 });
/// for await (const event of stream) {
///   console.log(event.count); // 0, 1, 2, ...
/// }
/// ```
async fn counter_subscription(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    input: CounterInput,
) -> RpcResult<EventStream<CounterEvent>> {
    let (tx, rx) = event_channel(32);

    tokio::spawn(async move {
        // Create the async stream using stream! macro
        let event_stream = stream! {
            let mut count = input.start;
            let mut ticker = interval(Duration::from_millis(input.interval_ms));

            loop {
                ticker.tick().await;

                // Check if reached max
                if count >= input.start + input.max_count {
                    break;
                }

                let event = CounterEvent {
                    count,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                yield Event::with_id(event, format!("counter-{}", count));
                count += 1;
            }
        };

        // Pin the stream and forward events to channel
        let mut pinned_stream = pin!(event_stream);
        while let Some(event) = pinned_stream.next().await {
            if sub_ctx.is_cancelled() {
                println!("Counter subscription cancelled");
                break;
            }
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(rx)
}

/// Stock price subscription using async_stream
async fn stock_subscription(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    input: StockInput,
) -> RpcResult<EventStream<StockPrice>> {
    let (tx, rx) = event_channel(32);

    tokio::spawn(async move {
        // Initialize base prices
        let mut prices: std::collections::HashMap<String, f64> = input
            .symbols
            .iter()
            .map(|s| {
                let base = match s.as_str() {
                    "AAPL" => 175.0,
                    "GOOGL" => 140.0,
                    "MSFT" => 380.0,
                    "AMZN" => 180.0,
                    "TSLA" => 250.0,
                    _ => 100.0,
                };
                (s.clone(), base)
            })
            .collect();

        let symbols = input.symbols.clone();
        let mut event_counter = 0u64;

        // Create async stream for stock prices
        let stock_stream = stream! {
            let mut ticker = interval(Duration::from_millis(1000));

            loop {
                ticker.tick().await;

                for symbol in &symbols {
                    if let Some(price) = prices.get_mut(symbol) {
                        // Simulate price change (-2% to +2%)
                        let change_percent = (rand_simple() - 0.5) * 4.0;
                        let change = *price * change_percent / 100.0;
                        *price += change;

                        event_counter += 1;

                        let event = StockPrice {
                            symbol: symbol.clone(),
                            price: (*price * 100.0).round() / 100.0,
                            change: (change * 100.0).round() / 100.0,
                            change_percent: (change_percent * 100.0).round() / 100.0,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };

                        yield Event::with_id(event, format!("stock-{}-{}", symbol, event_counter));
                    }
                }
            }
        };

        let mut pinned_stream = pin!(stock_stream);
        while let Some(event) = pinned_stream.next().await {
            if sub_ctx.is_cancelled() {
                break;
            }
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(rx)
}

/// Chat room subscription using async_stream
async fn chat_subscription(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    input: ChatRoomInput,
) -> RpcResult<EventStream<ChatMessage>> {
    let (tx, rx) = event_channel(32);
    let room_id = input.room_id.clone();

    // Log resumption if applicable
    if let Some(last_id) = &sub_ctx.last_event_id {
        println!("Resuming chat from event: {}", last_id);
    }

    tokio::spawn(async move {
        let users = ["Alice", "Bob", "Charlie", "Diana"];
        let messages = [
            "Hello everyone!",
            "How's it going?",
            "Working on something cool",
            "Anyone here?",
            "Check this out!",
            "That's awesome!",
            "I agree",
            "Let me think about it",
        ];

        let mut msg_counter = 0u64;
        let room_id_clone = room_id.clone();

        // Create async stream for chat messages
        let chat_stream = stream! {
            let mut ticker = interval(Duration::from_millis(2000));

            loop {
                ticker.tick().await;

                msg_counter += 1;
                let user_idx = (rand_simple() * users.len() as f64) as usize % users.len();
                let msg_idx = (rand_simple() * messages.len() as f64) as usize % messages.len();

                let message = ChatMessage {
                    id: format!("msg-{}", msg_counter),
                    room_id: room_id_clone.clone(),
                    user_id: users[user_idx].to_string(),
                    text: messages[msg_idx].to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                yield Event::with_id(message, format!("chat-{}-{}", room_id_clone, msg_counter));
            }
        };

        let mut pinned_stream = pin!(chat_stream);
        while let Some(event) = pinned_stream.next().await {
            if sub_ctx.is_cancelled() {
                break;
            }
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(rx)
}

/// Time subscription using async_stream - emits current time every second
async fn time_subscription(
    _ctx: Context<AppContext>,
    sub_ctx: SubscriptionContext,
    _input: (),
) -> RpcResult<EventStream<String>> {
    let (tx, rx) = event_channel(32);

    tokio::spawn(async move {
        let mut tick_count = 0u64;

        // Create async stream for time updates
        let time_stream = stream! {
            let mut ticker = interval(Duration::from_secs(1));

            loop {
                ticker.tick().await;
                tick_count += 1;

                let time = chrono::Utc::now().to_rfc3339();
                yield Event::with_id(time, format!("time-{}", tick_count));
            }
        };

        let mut pinned_stream = pin!(time_stream);
        while let Some(event) = pinned_stream.next().await {
            if sub_ctx.is_cancelled() {
                break;
            }
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(rx)
}

// =============================================================================
// Helpers
// =============================================================================

/// Simple pseudo-random number generator (0.0 to 1.0)
fn rand_simple() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f64 / u32::MAX as f64
}
