use lambda_http::{
    handler,
    lambda_runtime::{self, Context, Error},
    Request,
};

use serde_json::json;

use battlesnake_rs::{all_factories, BoxedFactory, Game};

use tracing_subscriber::EnvFilter;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let env_filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .json()
        .flatten_event(true)
        .init();

    let factories: Vec<_> = all_factories().into_iter().map(Arc::new).collect();

    lambda_runtime::run(handler(move |request: Request, context: Context| {
        let path = request.uri().path();
        let path_parts: Vec<&str> = path.split('/').filter(|x| x != &"").collect();
        let snake_name = path_parts.get(0).cloned();
        let factory = factories
            .iter()
            .find(|s| snake_name == Some(&s.name()))
            .cloned();

        api_move(factory, request, context)
    }))
    .await?;

    Ok(())
}

async fn api_move(
    factory: Option<Arc<BoxedFactory>>,
    request: Request,
    _context: Context,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let factory = factory.ok_or("Snake name not found")?;
    let path = request.uri().path();
    let path_parts: Vec<&str> = path.split('/').filter(|x| x != &"").collect();
    let action = path_parts.get(1);

    let string_body = if let lambda_http::Body::Text(s) = request.body() {
        Some(s)
    } else {
        None
    };

    match action {
        None => Ok(json!(factory.about())),
        Some(&"start") => Ok(json!("Nothing to do in start")),
        Some(&"end") | Some(&"move") => {
            let string_body = string_body.ok_or("Body was not a string")?;
            let state: Game = serde_json::from_str(string_body)?;
            let snake = factory.from_wire_game(state);

            match action {
                Some(&"end") => Ok(json!(snake.end())),
                Some(&"move") => Ok(serde_json::to_value(snake.make_move()?)?),
                _ => unreachable!("Nested matches mean this is impossible if bad code"),
            }
        }
        _ => Err("unknown-action".into()),
    }
}
