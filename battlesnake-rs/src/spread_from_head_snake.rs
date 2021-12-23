use crate::a_prime::APrimeCalculable;
use crate::flood_fill::spread_from_head::SpreadFromHead;
use crate::minimax::eval::EvalMinimaxSnake;
use crate::*;

use battlesnake_game_types::types::*;
use decorum::N64;

pub fn score<T>(node: &T) -> N64
where
    T::SnakeIDType: Copy,
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + SpreadFromHead
        + APrimeCalculable
        + HeadGettableGame
        + LengthGettableGame
        + FoodGettableGame,
{
    let square_counts = node.squares_per_snake();

    let my_space: f64 = (square_counts.get(node.you_id()).copied().unwrap_or(0) as u16).into();
    let total_space: f64 = (square_counts.values().sum::<usize>() as u16).into();

    // (
    //     N64::from(my_space / total_space),
    //     node.get_length_i64(node.you_id()),
    //     node.shortest_distance(
    //         &node.get_head_as_native_position(&node.you_id()),
    //         &node.get_all_food_as_native_positions(),
    //         None,
    //     )
    //     .map(|x| -x),
    // )
    N64::from(my_space / total_space)
}

pub struct SpreadFromHeadSnakeFactory;

impl BattlesnakeFactory for SpreadFromHeadSnakeFactory {
    fn name(&self) -> String {
        "spread-from-head".to_owned()
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
            color: Some("#0a86d8".to_owned()),
            head: None,
            tail: None,
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
