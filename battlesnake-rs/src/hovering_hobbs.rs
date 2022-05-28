use crate::a_prime::APrimeCalculable;
use crate::flood_fill::spread_from_head::SpreadFromHead;
use crate::*;

use battlesnake_game_types::compact_representation::{StandardCellBoard, WrappedCellBoard};
use battlesnake_game_types::types::*;
use battlesnake_minimax::EvalMinimaxSnake;
use decorum::N64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Score {
    LowOnHealth(Option<i32>, N64),
    FloodFill(N64),
}

pub fn score<BoardType, CellType>(node: &BoardType) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + SpreadFromHead<CellType>
        + APrimeCalculable
        + HeadGettableGame
        + HazardQueryableGame
        + HealthGettableGame
        + LengthGettableGame
        + FoodGettableGame,
{
    let square_counts = node.squares_per_snake_with_hazard_cost(5, 5);

    let me = node.you_id();
    let my_space: f64 = square_counts[me.as_usize()] as f64;
    let total_space: f64 = square_counts.iter().sum::<u16>() as f64;
    let my_ratio = N64::from(my_space / total_space);

    if node.get_health_i64(me) < 40 {
        let dist = node
            .shortest_distance(
                &node.get_head_as_native_position(me),
                &node.get_all_food_as_native_positions(),
                None,
            )
            .map(|x| -x);
        return Score::LowOnHealth(dist, my_ratio);
    }

    Score::FloodFill(my_ratio)
}

pub struct Factory;

impl BattlesnakeFactory for Factory {
    fn name(&self) -> String {
        "hovering-hobbs".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;
        let id_map = build_snake_id_map(&game);

        let name = "hovering-hobbs";

        if game_info.ruleset.name == "wrapped" {
            let game =
                WrappedCellBoard::<u16, { 19 * 21 }, 4>::convert_from_game(game, &id_map).unwrap();

            let snake = EvalMinimaxSnake::new(game, game_info, turn, &score, name);

            Box::new(snake)
        } else {
            let game =
                StandardCellBoard::<u16, { 19 * 21 }, 4>::convert_from_game(game, &id_map).unwrap();

            let snake = EvalMinimaxSnake::new(game, game_info, turn, &score, name);

            Box::new(snake)
        }
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#da8a1a".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            tail: Some("flame".to_owned()),
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
