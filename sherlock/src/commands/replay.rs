use std::{fs::read_to_string, net::SocketAddr, path::PathBuf};

use clap::Subcommand;
use color_eyre::eyre::Result;

#[derive(clap::Args, Debug)]
pub(crate) struct Replay {
    #[clap(subcommand)]
    command: ReplayCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum ReplayCommand {
    /// Start an engine that uses the local archive of Websocket Games
    Archive,
    /// Start the engine with a local file from the Rules repo output
    File(File),
}

#[derive(clap::Args, Debug)]
pub(crate) struct File {
    /// File to replay
    #[clap(value_parser)]
    file: PathBuf,
}

use axum::{
    extract::{
        ws::{rejection::WebSocketUpgradeRejection, Message, WebSocket, WebSocketUpgrade},
        Path,
    },
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::Value;
use tower_http::cors::CorsLayer;

use crate::websockets::rules_format_to_websocket;

async fn game_handler(Path(game_id): Path<String>) -> Response {
    println!("We got a game for {game_id}");

    let (info, _frames, _end_frame) = rules_format_to_websocket(
        read_to_string("/Users/coreyja/Downloads/group_0_game_0.jsonl.txt").unwrap(),
    );

    let game_info = if game_id == ":local:" {
        Ok(serde_json::to_string(&info).unwrap())
    } else {
        read_to_string(format!("./archive/{game_id}/info.json"))
    };

    match game_info {
        Ok(info) => IntoResponse::into_response(Json::<Value>(
            serde_json::from_str(&info)
                .expect("This should be safe since we just deserialized from json"),
        )),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    }
}

async fn websocket_handler(
    ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    Path(game_id): Path<String>,
) -> Response {
    println!("Websocket for game {game_id}");

    let (_info, frames, end_frame) = rules_format_to_websocket(
        read_to_string("/Users/coreyja/Downloads/group_0_game_0.jsonl.txt").unwrap(),
    );

    let game_lines = if game_id == ":local:" {
        let mut lines: Vec<String> = vec![];
        lines.extend(frames.iter().map(|s| serde_json::to_string(s).unwrap()));
        lines.push(serde_json::to_string(&end_frame).unwrap());

        Ok(lines.join("\n"))
    } else {
        read_to_string(format!("./archive/{game_id}/websockets.jsonl"))
    };
    match game_lines {
        Ok(l) => match ws {
            Ok(ws) => ws.on_upgrade(move |s| handle_socket(s, l)),
            Err(_) => "fallback".into_response(),
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    }
}

async fn handle_socket(mut socket: WebSocket, lines: String) {
    for line in lines.lines() {
        let message: Message = Message::Text(line.to_string());

        if socket.send(message).await.is_err() {
            return;
        }
    }

    let _ = socket.send(Message::Close(None)).await;
    println!("Closing websocket connection");
}

impl Replay {
    pub(crate) fn run(self) -> Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                println!("Hello world");

                let addr = SocketAddr::from(([0, 0, 0, 0], 8085));

                let cors = CorsLayer::new()
                    .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
                    .allow_origin(tower_http::cors::Any)
                    .allow_credentials(false);

                let app = Router::new()
                    .route("/games/:game_id", get(game_handler))
                    .route("/games/:game_id/events", get(websocket_handler))
                    .layer(cors);

                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });

        Ok(())
    }
}
