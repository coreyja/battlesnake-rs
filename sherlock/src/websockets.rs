use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    #[serde(rename = "X")]
    x: u32,
    #[serde(rename = "Y")]
    y: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Snake {
    #[serde(rename = "Author")]
    author: String,
    #[serde(rename = "Body")]
    body: Vec<Point>,
    #[serde(rename = "Color")]
    color: String,
    #[serde(rename = "Death")]
    death: Option<String>,
    #[serde(rename = "Error")]
    error: Option<String>,
    #[serde(rename = "HeadType")]
    head_type: String,
    #[serde(rename = "Health")]
    health: u32,
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "IsBot")]
    is_bot: bool,
    #[serde(rename = "IsEnvironment")]
    is_false: bool,
    #[serde(rename = "Latency")]
    latency: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Shout")]
    shout: Option<String>,
    #[serde(rename = "Squad")]
    squad: Option<String>,
    #[serde(rename = "StatusCode")]
    status_code: u32,
    #[serde(rename = "TailType")]
    tail_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Frame {
    #[serde(rename = "Food")]
    food: Vec<Point>,
    #[serde(rename = "Hazards")]
    hazards: Vec<Point>,
    #[serde(rename = "Snakes")]
    snakes: Vec<Snake>,
    #[serde(rename = "Turn")]
    turn: u32,
}

pub(crate) fn get_websockets_for_game(game_id: &str) -> Result<Vec<String>> {
    let url = Url::parse(&format!(
        "wss://engine.battlesnake.com/games/{game_id}/events"
    ))?;
    let (mut socket, _response) = connect(url)?;

    let mut messages = vec![];
    while let Ok(Message::Text(msg)) = socket.read_message() {
        messages.push(msg);
    }

    Ok(messages)
}
