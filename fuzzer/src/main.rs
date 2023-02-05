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

fn about_equal_snake(
    this: &BattleSnake,
    compact: &StandardCellBoard4Snakes11x11,
    id_map: &HashMap<String, SnakeId>,
) {
    let id = this.id.clone();
    let compact_id = id_map.get(&id).unwrap();
    let other: Vec<Position> = compact
        .get_snake_body_vec(&compact_id)
        .into_iter()
        .map(|c| c.into_position(11))
        .collect();

    assert_eq!(
        this.body[0], this.head,
        "Snake {} head is not at the front of the body from Go",
        id
    );

    let body_vec: Vec<_> = this.body.iter().cloned().collect();

    assert_eq!(
        this.head, other[0],
        "Snake {id} first body piece does not match its head in Rust"
    );

    assert_eq!(body_vec, other, "Snake {id} body does not match");
}

fn assert_about_equal(
    this: &Game,
    other: &StandardCellBoard4Snakes11x11,
    id_map: &HashMap<String, SnakeId>,
) {
    // println!("Go:\n{}", this);
    // println!("Rust:\n{}", other);

    let snake_ids = other.get_snake_ids();

    assert_eq!(this.board.height, other.get_height());
    assert_eq!(this.board.width, other.get_width());

    assert_eq!(this.board.snakes.len(), snake_ids.len());

    for snake in &this.board.snakes {
        let snake_id = id_map.get(&snake.id).unwrap();

        assert_eq!(snake.health, other.get_health(&snake_id) as i32);
        about_equal_snake(&snake, &other, &id_map);
    }
}

#[derive(Serialize, Debug, PartialEq)]
struct SimulateMove {
    id: String,
    r#move: String,
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
        .map(|(id, r#move)| SimulateMove {
            id,
            r#move: r#move.to_string(),
        })
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
    let mut rng = rand::thread_rng();
    let client = reqwest::Client::new();

    let mut rounds = 0;

    loop {
        let mut game = fuzz::random_game();
        let id_map = build_snake_id_map(&game);
        let you_id = game.you_id().to_owned();

        while !game.is_over() {
            let moves: Vec<_> = game
                .random_reasonable_move_for_each_snake(&mut rng)
                .collect();

            // println!("Before:\nMoves: {moves:?}\n{game}");

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
            assert_about_equal(&fake_game, &rust_result, &id_map);

            rounds += 1;
            game.board = official_result;

            let you = game.board.snakes.iter().find(|s| s.id == you_id);

            if let Some(you) = you {
                game.you = you.clone();
            } else {
                game.you.health = 0;
            }

            if rounds % 10_000 == 0 {
                println!("Rounds: {}", rounds);
            }
        }
    }

    Ok(())
}
