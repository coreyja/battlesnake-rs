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

use std::fmt::Debug;

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
