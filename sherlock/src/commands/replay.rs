use std::{fs::read_to_string, net::SocketAddr};

use color_eyre::eyre::Result;

#[derive(clap::Args, Debug)]
pub(crate) struct Replay;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ReplayQueryParams {
    game_id: String,
}

async fn handler(ws: WebSocketUpgrade, query: Query<ReplayQueryParams>) -> Response {
    let game_id = query.game_id.clone();
    let game_lines = read_to_string(format!("./archive/{game_id}.jsonl"));
    match game_lines {
        Ok(l) => ws.on_upgrade(move |s| handle_socket(s, l)),
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

                let app = Router::new().route("/ws", get(handler));

                let addr = SocketAddr::from(([0, 0, 0, 0], 8085));

                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });

        Ok(())
    }
}
