use battlesnake_rs::{Game, Move, RandomReasonableMovesGame, VictorDeterminableGame};
use serde::{Deserialize, Serialize};

use color_eyre::Result;
use types::wire_representation::BattleSnake;

mod fuzz;

fn about_equal_snake(this: &BattleSnake, other: &BattleSnake) -> bool {
    this.id == other.id
        && this.health == other.health
        && this.head == other.head
        && this.body == other.body
}

fn about_equal(this: &Game, other: &Game) -> bool {
    this.board.height == other.board.height
        && this.board.width == other.board.width
        && this.board.snakes.len() == other.board.snakes.len()
        && this.board.snakes.iter().all(|snake| {
            other
                .board
                .snakes
                .iter()
                .any(|other_snake| about_equal_snake(snake, other_snake))
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
) -> Result<Game> {
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
        .json::<Game>()
        .await?;

    Ok(res)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut game = fuzz::random_game();
    let mut turns = 0;
    let mut rng = rand::thread_rng();

    let client = reqwest::Client::new();

    while !game.is_over() {
        println!("{}", game);

        let moves: Vec<_> = game
            .random_reasonable_move_for_each_snake(&mut rng)
            .collect();

        let official_result = official_simulate_moves(&client, &game, moves.clone()).await?;

        turns += 1;
        game = official_result;
    }

    Ok(())
}
