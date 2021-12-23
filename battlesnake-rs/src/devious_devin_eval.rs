use crate::devious_devin_mutable::score;
use crate::minimax::eval::EvalMinimaxSnake;
use crate::*;

use battlesnake_game_types::types::*;

pub struct Factory;

impl BattlesnakeFactory for Factory {
    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;
        let id_map = build_snake_id_map(&game);

        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let snake = EvalMinimaxSnake::new(game, game_info, turn, &score);

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
