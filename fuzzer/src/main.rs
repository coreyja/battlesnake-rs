use std::collections::HashMap;

use battlesnake_rs::{
    build_snake_id_map, improbable_irene::Instrument, Game, HeadGettableGame, HealthGettableGame,
    Move, RandomReasonableMovesGame, SimulableGame, SizeDeterminableGame, SnakeBodyGettableGame,
    SnakeId, StandardCellBoard4Snakes11x11, VictorDeterminableGame,
};
use serde::{Deserialize, Serialize};

use color_eyre::Result;
use types::{
    types::{SnakeIDGettableGame, YouDeterminableGame},
    wire_representation::{BattleSnake, Board, Position},
};

mod fuzz;

fn about_equal_snake(this: &BattleSnake, other: &Vec<Position>) -> bool {
    let body_vec: Vec<_> = this.body.iter().cloned().collect();
    &body_vec == other
}

fn about_equal(
    this: &Game,
    other: &StandardCellBoard4Snakes11x11,
    id_map: &HashMap<String, SnakeId>,
) -> bool {
    println!("Go:\n{}", this);
    println!("Rust:\n{}", other);

    let snake_ids = other.get_snake_ids();

    this.board.height == other.get_height()
        && this.board.width == other.get_width()
        && this.board.snakes.len() == other.get_snake_ids().len()
        && this.board.snakes.iter().all(|snake| {
            let snake_id = id_map.get(&snake.id).unwrap();
            snake.health == other.get_health(&snake_id) as i32
                && snake.head == other.get_head_as_position(&snake_id)
                && snake_ids.iter().any(|id| {
                    let body = other
                        .get_snake_body_iter(id)
                        .map(|p| p.into_position(11))
                        .collect();
                    about_equal_snake(snake, &body)
                })
        })
}

#[derive(Serialize, Debug, PartialEq)]
struct SimulateMove {
    id: String,
    r#move: Move,
}

#[derive(Serialize, Debug, PartialEq)]
struct SimulateRequest {
    game: Game,
    moves: Vec<SimulateMove>,
}

async fn official_simulate_moves(
    client: &reqwest::Client,
    orig: &Game,
    moves: Vec<(String, Move)>,
) -> Result<Board> {
    let moves = moves
        .into_iter()
        .map(|(id, r#move)| SimulateMove { id, r#move })
        .collect();
    let body = SimulateRequest {
        game: orig.clone(),
        moves,
    };
    let res = client
        .post("http://localhost:8090/simulate")
        .json(&body)
        .send()
        .await?
        .json::<Board>()
        .await?;

    Ok(res)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut game = fuzz::random_game();
    let mut turns = 0;
    let mut rng = rand::thread_rng();

    let you_id = game.you_id().to_owned();

    let client = reqwest::Client::new();

    let id_map = build_snake_id_map(&game);

    while !game.is_over() {
        let moves: Vec<_> = game
            .random_reasonable_move_for_each_snake(&mut rng)
            .collect();

        println!("Before:\nMoves: {moves:?}\n{game}");

        let official_result = official_simulate_moves(&client, &game, moves.clone())
            .await
            .unwrap();

        // TODO: Compare with Rust
        let compact =
            StandardCellBoard4Snakes11x11::convert_from_game(game.clone(), &id_map).unwrap();
        let compact_moves: Vec<(SnakeId, Vec<Move>)> = moves
            .iter()
            .map(|(id, m)| (*id_map.get(id).unwrap(), vec![*m]))
            .collect();
        let mut results = compact.simulate_with_moves(&Instrument {}, compact_moves);
        let rust_result = results.next().unwrap().1;

        let mut fake_game = game.clone();
        fake_game.board = official_result.clone();
        assert!(about_equal(&fake_game, &rust_result, &id_map));

        turns += 1;
        game.board = official_result;

        let you = game.board.snakes.iter().find(|s| s.id == you_id);

        if let Some(you) = you {
            game.you = you.clone();
        } else {
            game.you.health = 0;
        }
    }

    Ok(())
}
