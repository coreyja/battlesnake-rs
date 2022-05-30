use crate::flood_fill::jump_flooding::JumpFlooding;
use crate::*;

use battlesnake_minimax::paranoid::MinimaxSnake;

use battlesnake_game_types::compact_representation::WrappedCellBoard4Snakes11x11;
use battlesnake_game_types::types::*;

use decorum::N64;

pub fn score<T>(node: &T) -> N64
where
    T::SnakeIDType: Copy,
    T: SnakeIDGettableGame + YouDeterminableGame + JumpFlooding,
{
    let square_counts = node.squares_per_snake();

    let my_space: f64 = (square_counts.get(node.you_id()).copied().unwrap_or(0) as u16).into();
    let total_space: f64 = (square_counts.values().sum::<usize>() as u16).into();

    N64::from(my_space / total_space)
}

pub struct JumpFloodingSnakeFactory;

impl BattlesnakeFactory for JumpFloodingSnakeFactory {
    fn name(&self) -> String {
        "jump-flooding".to_owned()
    }

    fn create_from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;
        let id_map = build_snake_id_map(&game);

        let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let snake = MinimaxSnake::new(game, game_info, turn, &score, "jump-flooding");

        Box::new(snake)
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#efae09".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            tail: None,
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
