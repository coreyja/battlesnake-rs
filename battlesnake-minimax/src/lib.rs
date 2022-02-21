#![deny(
    warnings,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs
)]
//! This crate implements the minimax algorithm for the battlesnake game. You provide a 'scoring'
//! function that turns a given board into anything that implements the `Ord` trait.
//!
//! We lean on the `battlesnake-game-types` crate for the game logic, and in particular for the
//! simulate logic, which is used to generate the next board states.

mod eval;

pub use eval::{EvalMinimaxSnake, MoveOutput};
