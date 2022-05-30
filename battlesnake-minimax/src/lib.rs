#![deny(
    warnings,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs
)]
//! This crate implements the minimax algorithm for the battlesnake game. You provide a 'scoring'
//! function that turns a given board into anything that implements the `Ord` trait.
//!
//! There are multiple variants to multiplayer minimax. This crate currently only supports the
//! `paranoid` variant, which can be found in the [paranoid] module
//! For more information check out my [Minimax Blog Post](https://coreyja.com/BattlesnakeMinimax/Minimax%20in%20Battlesnake/)
//!
//! We lean on the [battlesnake_game_types] crate for the game logic, and in particular for the
//! simulate logic, which is used to generate the next board states.
//!
//! ```rust
//! use std::time::Duration;
//! use battlesnake_minimax::paranoid::{MinMaxReturn, EvalMinimaxSnake, SnakeOptions};
//! use battlesnake_game_types::{types::build_snake_id_map, compact_representation::StandardCellBoard4Snakes11x11, wire_representation::Game};
//!
//! // This fixture data matches what we expect to come from the Battlesnake Game Server
//! let game_state_from_server = include_str!("../../../battlesnake-rs/fixtures/start_of_game.json");
//!
//! // First we take the JSON from the game server and construct a `Game` struct which
//! // represents the 'wire' representation of the game state
//! let wire_game: Game = serde_json::from_str(game_state_from_server).unwrap();
//!
//! // The 'compact' representation of the game state doesn't include the game_info but we use
//! // it for some of our tracing so we want to clone it before we create the compact representation
//! let game_info = wire_game.game.clone();
//!
//! let snake_id_map = build_snake_id_map(&wire_game);
//! let compact_game = StandardCellBoard4Snakes11x11::convert_from_game(wire_game, &snake_id_map).unwrap();
//!
//! // This is the scoring function that we will use to evaluate the game states
//! // Here it just returns a constant but would ideally contain some logic to decide which
//! // states are better than others
//! fn score_function(board: &StandardCellBoard4Snakes11x11) -> i32 { 4 }
//!
//! // Optional settings for the snake
//! let snake_options = SnakeOptions {
//!   network_latency_padding: Duration::from_millis(100),
//!   ..Default::default()
//! };
//!
//!
//! let minimax_snake = EvalMinimaxSnake::new_with_options(
//!    compact_game,
//!    game_info,
//!    0,
//!    &score_function,
//!    "minimax_snake",
//!    snake_options,
//! );
//!
//! // Now we can use the minimax snake to generate the next move!
//! // Here we use the function [EvalMinimaxSnake::deepened_minimax_until_timelimit] to run the minimax
//! // algorithm until the time limit specified in the give game
//! let result: MinMaxReturn<_, _> = minimax_snake.deepened_minimax_until_timelimit(snake_id_map.values().cloned().collect());
//! ```

pub use battlesnake_game_types;

pub mod paranoid;

/// The move output to be returned to the Battlesnake Engine
#[derive(Debug, Clone)]
pub struct MoveOutput {
    /// A stringified move
    pub r#move: String,
    /// An optional shout that will be echoed to you on your next turn
    pub shout: Option<String>,
}

#[derive(Debug, Clone, Copy)]
/// Any empty struct that implements `SimulatorInstruments` as a no-op which can be used when you don't want
/// to time the simulation
pub struct Instruments {}

#[cfg(test)]
mod tests {}
