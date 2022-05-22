//! There are multiple multiplayer variations to minimax, this module is for the `paranoid`
//! variant. [This is currently the only variant supported by this trait]
//!
//! This variant assumes all your opponents are working together to minimize your score. The
//! implementation uses Alpha-Beta pruning to be efficient
//!
//! For more information about Paranoid Minimax in Battlesnake see this accompanying blog post
//! <https://coreyja.com/BattlesnakeMinimax/Minimax%20in%20Battlesnake/>

mod score;
pub use score::{Scorable, WrappedScore};

mod minimax_return;
pub use minimax_return::MinMaxReturn;

mod eval;
pub use eval::EvalMinimaxSnake;
