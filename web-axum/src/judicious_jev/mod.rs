//! Async TypeSafe snake. Kept outside the synchronous BattlesnakeAI factory API.

mod board;
pub(super) mod telemetry;

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use battlesnake_rs::{Game, MoveOutput};
use color_eyre::eyre::{Result, WrapErr, eyre};
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Semaphore;
use tracing::instrument::WithSubscriber;

const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const NETWORK_RESERVE: Duration = Duration::from_millis(50);
const MAX_API_TIME: Duration = Duration::from_millis(450);

struct Jev {
    client: Client,
    endpoint: String,
    model: String,
    enabled: bool,
    in_flight: Arc<Semaphore>,
}

impl Jev {
    fn new(key: Option<&str>, endpoint: &str, model: String) -> Result<Self> {
        let key = key.filter(|key| !key.trim().is_empty());
        let mut headers = HeaderMap::new();
        if let Some(key) = key {
            let mut authorization = HeaderValue::from_str(&format!("Bearer {}", key.trim()))
                .wrap_err("Invalid TYPESAFE_API_KEY header")?;
            authorization.set_sensitive(true);
            headers.insert(AUTHORIZATION, authorization);
        }
        let client = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(MAX_API_TIME)
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            client,
            endpoint: endpoint.into(),
            model,
            enabled: key.is_some(),
            in_flight: Arc::new(Semaphore::new(32)),
        })
    }

    async fn choose(&self, game: &Game, moves: &[board::Candidate]) -> Result<Choice> {
        let criteria: BTreeMap<_, _> = moves
            .iter()
            .map(|m| {
                (
                    m.direction,
                    format!(
                        "Move {} to ({}, {}).",
                        m.direction, m.destination.x, m.destination.y
                    ),
                )
            })
            .collect();
        // Only game mechanics go to the model: opponent names and shouts are irrelevant.
        let snakes: Vec<_> = game
            .board
            .snakes
            .iter()
            .map(|s| {
                json!({
                    "is_you": s.id == game.you.id, "health": s.health,
                    "length": s.body.len(), "body": s.body,
                })
            })
            .collect();
        let request = json!({
            "model": self.model,
            "state": {
                "turn": game.turn,
                "health": game.you.health,
                "length": game.you.body.len(),
                "wrapped": game.is_wrapped(),
                "board": {"width": game.board.width, "height": game.board.height,
                    "food": game.board.food, "hazards": game.board.hazards, "snakes": snakes},
                "moves": moves,
            },
            "questions": {"move": {
                "type": "choice",
                "instructions": include_str!("instructions.txt"),
                "criteria": criteria,
            }},
        });
        // No retries: a late answer cannot help this turn.
        let started = Instant::now();
        let response = async {
            self.client
                .post(&self.endpoint)
                .json(&request)
                .send()
                .await?
                .error_for_status()?
                .json::<Evaluation>()
                .await
        }
        .await;
        telemetry::record_api(game, &self.model, started.elapsed(), response.as_ref().ok());
        let response = response?;
        let choice = serde_json::from_value::<Answers>(response.answers)?.r#move;
        if !moves.iter().any(|m| m.direction == choice.choice)
            || !choice.confidence.is_finite()
            || !(0.0..=1.0).contains(&choice.confidence)
        {
            return Err(eyre!("Invalid Jev move choice"));
        }
        Ok(choice)
    }

    async fn make_move(self: &Arc<Self>, game: &Game) -> MoveOutput {
        let started = Instant::now();
        let moves = board::candidates(game);
        let fallback = board::fallback(game, &moves);
        let turn_time = Duration::from_millis(game.game.timeout.max(0) as u64);
        let budget = turn_time
            .saturating_sub(NETWORK_RESERVE)
            .saturating_sub(started.elapsed())
            .min(MAX_API_TIME);
        let reason = if !self.enabled {
            "no_api_key"
        } else if moves.len() <= 1 {
            "forced_move"
        } else if budget.is_zero() {
            "no_time"
        } else if let Ok(permit) = self.in_flight.clone().try_acquire_owned() {
            let jev = Arc::clone(self);
            let request_game = game.clone();
            // Keep the request alive after a turn timeout to capture late billable
            // usage. Client timeout and the semaphore bound background work.
            let request = tokio::spawn(
                async move {
                    let _permit = permit;
                    jev.choose(&request_game, &moves).await
                }
                .with_current_subscriber(),
            );
            match tokio::time::timeout(budget, request).await {
                Ok(Ok(Ok(answer))) => {
                    telemetry::record_move(game, &answer.choice, "jev", started.elapsed());
                    tracing::debug!(game_id = game.game.id, turn = game.turn,
                        confidence = answer.confidence, probabilities = ?answer.probabilities,
                        "Jev choice distribution");
                    return MoveOutput {
                        r#move: answer.choice,
                        shout: Some("A considered choice.".into()),
                    };
                }
                Ok(Ok(Err(error))) => {
                    tracing::warn!(snake = "judicious-jev", %error, "Jev request failed");
                    "api_error"
                }
                Ok(Err(error)) => {
                    tracing::warn!(%error, "Jev request task failed");
                    "task_error"
                }
                Err(_) => "timeout",
            }
        } else {
            "at_capacity"
        };
        telemetry::record_move(game, fallback, reason, started.elapsed());
        MoveOutput {
            r#move: fallback.into(),
            shout: Some("Going with my gut.".into()),
        }
    }
}

#[derive(Deserialize)]
struct Evaluation {
    // Parse usage before interpreting the answer, so malformed choices do not
    // hide a response's reported token consumption.
    answers: Value,
    model: String,
    usage: Usage,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Deserialize)]
struct Answers {
    r#move: Choice,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename = "choice")]
struct Choice {
    choice: String,
    confidence: f64,
    probabilities: BTreeMap<String, f64>,
}

pub(super) fn router<S: Clone + Send + Sync + 'static>() -> Result<Router<S>> {
    let key = std::env::var("TYPESAFE_API_KEY").ok();
    let model = std::env::var("TYPESAFE_MODEL").unwrap_or_else(|_| "jev-latest".into());
    let jev = Jev::new(key.as_deref(), ENDPOINT, model)?;
    if !jev.enabled {
        tracing::warn!("TYPESAFE_API_KEY is unset; Judicious Jev will use local fallback moves");
    }
    Ok(routes(Arc::new(jev)))
}

fn routes<S: Clone + Send + Sync + 'static>(jev: Arc<Jev>) -> Router<S> {
    Router::new()
        .route("/judicious-jev", get(info))
        .route("/judicious-jev/start", post(start))
        .route("/judicious-jev/end", post(end))
        .route("/judicious-jev/move", post(make_move))
        .with_state(jev)
}

async fn info() -> Json<Value> {
    Json(
        json!({"apiversion": "1", "author": "coreyja", "color": "#14b8a6",
        "head": "trans-rights-scarf", "tail": "default", "version": env!("CARGO_PKG_VERSION")}),
    )
}

async fn make_move(State(jev): State<Arc<Jev>>, Json(game): Json<Game>) -> Json<MoveOutput> {
    Json(jev.make_move(&game).await)
}

async fn start(Json(game): Json<Game>) -> StatusCode {
    tracing::info!(
        event_kind = "jev_game_start",
        game_id = game.game.id,
        snake_id = game.you.id,
        "Judicious Jev started a game"
    );
    StatusCode::NO_CONTENT
}

async fn end(Json(game): Json<Game>) -> StatusCode {
    let won = game.board.snakes.len() == 1 && game.board.snakes[0].id == game.you.id;
    tracing::info!(
        event_kind = "jev_game_end",
        game_id = game.game.id,
        snake_id = game.you.id,
        turn = game.turn,
        won,
        "Judicious Jev finished a game"
    );
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests;
