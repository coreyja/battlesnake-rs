use battlesnake_game_types::{
    types::SizeDeterminableGame,
    wire_representation::{BattleSnake, Game, NestedGame, Position},
};
use color_eyre::eyre::Result;
use itertools::Itertools;
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
    is_environment: bool,
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

#[derive(Serialize, Debug)]
pub(crate) struct WrappedFrame {
    #[serde(rename = "Type")]
    t: String,
    #[serde(rename = "Data")]
    data: Frame,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EndFrame {
    #[serde(rename = "Type")]
    t: String,
    #[serde(rename = "Data")]
    data: GameInfo,
}

impl From<Frame> for WrappedFrame {
    fn from(f: Frame) -> Self {
        Self {
            t: "frame".into(),
            data: f,
        }
    }
}

impl From<&Position> for Point {
    fn from(p: &Position) -> Self {
        Point {
            x: p.x as u32,
            y: p.y as u32,
        }
    }
}

impl From<&BattleSnake> for Snake {
    fn from(s: &BattleSnake) -> Self {
        Snake {
            author: "Snakedev".into(),
            body: s.body.iter().map(|p| p.into()).collect_vec(),
            color: "".into(),
            death: None,
            error: None,
            head_type: "".into(),
            health: s.health as u32,
            id: s.id.clone(),
            is_bot: false,
            is_environment: false,
            latency: "".into(),
            name: s.name.clone(),
            shout: s.shout.clone(),
            squad: None,
            status_code: 200,
            tail_type: "".into(),
        }
    }
}

fn frame_from_game(input: Game, turn: u32) -> Frame {
    let snakes: Vec<Snake> = input.board.snakes.iter().map(|s| s.into()).collect();
    let hazards = vec![];
    let food = vec![];

    Frame {
        snakes,
        food,
        hazards,
        turn,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameRuleset {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameGame {
    #[serde(rename = "Ruleset")]
    ruleset: FrameRuleset,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameInfo {
    #[serde(rename = "Game")]
    game: FrameGame,
    #[serde(rename = "Height")]
    height: u32,
    #[serde(rename = "Width")]
    width: u32,
}

pub(crate) fn rules_format_to_websocket(input: String) -> (GameInfo, Vec<WrappedFrame>, EndFrame) {
    let mut lines = input.lines();

    let header = lines.next().unwrap();
    let inner_game: NestedGame = serde_json::from_str(header).unwrap();

    let mut rest = lines.collect_vec();
    let first_game = serde_json::from_str::<Game>(rest[0]).unwrap();

    let _footer = rest.pop().unwrap();

    let frames: Vec<WrappedFrame> = rest
        .into_iter()
        .map(|s| serde_json::from_str::<Game>(s).unwrap())
        .enumerate()
        .map(|(i, g)| frame_from_game(g, i as u32).into())
        .collect_vec();

    let game_info = GameInfo {
        game: FrameGame {
            ruleset: FrameRuleset {
                name: inner_game.ruleset.name,
            },
        },
        height: first_game.get_height(),
        width: first_game.get_width(),
    };

    let end = EndFrame {
        t: "game_end".into(),
        data: game_info.clone(),
    };

    (game_info, frames, end)
}

pub(crate) fn get_raw_messages_from_game(game_id: &str) -> Result<Vec<String>> {
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
