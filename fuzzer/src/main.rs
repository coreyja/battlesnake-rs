use battlesnake_rs::{Game, Move, RandomReasonableMovesGame, VictorDeterminableGame};

mod fuzz;

async fn official_simulate_moves(orig: &Game, moves: Vec<(String, Move)>) -> Game {
    todo!("official_simulate_moves")
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut game = fuzz::random_game();
    let mut turns = 0;
    let mut rng = rand::thread_rng();

    while !game.is_over() {
        println!("{}", fuzz::random_game());

        let moves: Vec<_> = game
            .random_reasonable_move_for_each_snake(&mut rng)
            .collect();

        let official_result = official_simulate_moves(&game, moves.clone()).await;

        turns += 1;
        game = official_result;
    }
}
