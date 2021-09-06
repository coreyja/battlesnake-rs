use lambda_http::{
    handler,
    lambda_runtime::{self, Context, Error},
    Request,
};

use serde_json::json;

use battlesnake_rs::devious_devin::DeviousDevin;
use battlesnake_rs::{amphibious_arthur::AmphibiousArthur, famished_frank::FamishedFrank};
use battlesnake_rs::{bombastic_bob::BombasticBob, eremetic_eric::EremeticEric};
use battlesnake_rs::{constant_carter::ConstantCarter, gigantic_george::GiganticGeorge};
use battlesnake_rs::{BoxedSnake, Game};

use tracing_subscriber::layer::SubscriberExt;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let subscriber = tracing_subscriber::registry::Registry::default()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(tracing_subscriber::fmt::Layer::default());

    tracing::subscriber::set_global_default(subscriber).expect("setting global default failed");

    let snakes: Vec<Arc<BoxedSnake>> = vec![
        Arc::new(Box::new(AmphibiousArthur {})),
        Arc::new(Box::new(BombasticBob {})),
        Arc::new(Box::new(ConstantCarter {})),
        Arc::new(Box::new(DeviousDevin {})),
        Arc::new(Box::new(EremeticEric {})),
        Arc::new(Box::new(FamishedFrank {})),
        Arc::new(Box::new(GiganticGeorge {})),
    ];

    lambda_runtime::run(handler(move |request: Request, context: Context| {
        let path = request.uri().path();
        let path_parts: Vec<&str> = path.split('/').filter(|x| x != &"").collect();
        let snake_name = path_parts.get(0).cloned();
        let snake = snakes
            .iter()
            .cloned()
            .find(|s| snake_name == Some(&s.name()));

        api_move(snake, request, context)
    }))
    .await?;

    Ok(())
}

async fn api_move(
    snake: Option<Arc<BoxedSnake>>,
    request: Request,
    _context: Context,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let snake = snake.ok_or("Snake name not found")?;
    let path = request.uri().path();
    let path_parts: Vec<&str> = path.split('/').filter(|x| x != &"").collect();
    let action = path_parts.get(1);

    let string_body = if let lambda_http::Body::Text(s) = request.body() {
        Some(s)
    } else {
        None
    };

    match action {
        None => Ok(json!(snake.about())),
        Some(&"start") => Ok(json!(snake.start())),
        Some(&"end") => {
            let string_body = string_body.ok_or("Body was not a string")?;
            let state: Game = serde_json::from_str(string_body)?;
            Ok(json!(snake.end(state)))
        }
        Some(&"move") => {
            let string_body = string_body.ok_or("Body was not a string")?;
            let state: Game = serde_json::from_str(string_body)?;
            Ok(serde_json::to_value(snake.make_move(state)?)?)
        }
        _ => Err("unknown-action".into()),
    }
}
