use crate::devious_devin_mutable::{score, ScoreEndState};
use crate::minimax::eval::EvalMinimaxSnake;
use crate::*;

use battlesnake_game_types::types::*;

pub struct Factory;

impl Factory {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, game: Game) -> EvalMinimaxSnake<CellBoard4Snakes11x11, ScoreEndState> {
        let game_info = game.game.clone();
        let turn = game.turn;
        let id_map = build_snake_id_map(&game);

        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        EvalMinimaxSnake::new(game, game_info, turn, &score)
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

impl BattlesnakeFactory for Factory {
    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        let snake = self.create(game);

        Box::new(snake)
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#efae09".to_owned()),
            head: None,
            tail: None,
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
