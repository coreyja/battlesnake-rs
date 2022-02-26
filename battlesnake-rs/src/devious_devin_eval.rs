use crate::a_prime::{APrimeCalculable, ClosestFoodCalculable};
use crate::*;
use battlesnake_minimax::EvalMinimaxSnake;

use battlesnake_game_types::types::*;

use battlesnake_game_types::compact_representation::StandardCellBoard4Snakes11x11;

pub struct Factory;

#[derive(Serialize, PartialEq, PartialOrd, Ord, Eq, Debug, Copy, Clone)]
pub enum ScoreEndState {
    /// depth: i64
    Lose(i64),
    /// depth: i64
    Tie(i64),
    /// difference_in_snake_length: u16, negative_distance_to_nearest_food: Option<i32>, health: u8
    ShorterThanOpponent(i64, Option<i32>, i64),
    /// negative_distance_to_opponent: Option<i64>, difference_in_snake_length: i64, health: u8
    LongerThanOpponent(Option<i32>, i64, i64),
    /// depth: i64
    Win(i64),
}

impl ScoreEndState {
    pub fn terminal_depth(&self) -> Option<i64> {
        match &self {
            ScoreEndState::Win(d) => Some(-d),
            ScoreEndState::Tie(d) | ScoreEndState::Lose(d) => Some(*d),
            _ => None,
        }
    }
}

pub fn score<
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + HeadGettableGame
        + APrimeCalculable
        + FoodGettableGame,
>(
    node: &T,
) -> ScoreEndState {
    let me_id = node.you_id();
    let opponents: Vec<T::SnakeIDType> = node
        .get_snake_ids()
        .into_iter()
        .filter(|x| x != me_id)
        .collect();

    let opponent_heads: Vec<_> = opponents
        .iter()
        .map(|s| node.get_head_as_native_position(s))
        .collect();
    let my_head = node.get_head_as_native_position(me_id);

    let my_length = node.get_length_i64(me_id);

    let max_opponent_length = opponents
        .iter()
        .map(|o| node.get_length_i64(o))
        .max()
        .unwrap();
    let length_difference = (my_length as i64) - (max_opponent_length as i64);
    let my_health = node.get_health_i64(me_id);

    if max_opponent_length >= my_length || my_health < 20 {
        let negative_closest_food_distance = node.dist_to_closest_food(&my_head, None).map(|x| -x);

        return ScoreEndState::ShorterThanOpponent(
            length_difference,
            negative_closest_food_distance,
            my_health.max(50),
        );
    }

    let negative_distance_to_opponent = node
        .shortest_distance(&my_head, &opponent_heads, None)
        .map(|dist| -dist);

    ScoreEndState::LongerThanOpponent(
        negative_distance_to_opponent,
        length_difference.max(4),
        my_health.max(50),
    )
}

impl Factory {
    pub fn new() -> Self {
        Self
    }

    pub fn create(
        &self,
        game: Game,
    ) -> EvalMinimaxSnake<StandardCellBoard4Snakes11x11, ScoreEndState, 4> {
        let game_info = game.game.clone();
        let turn = game.turn;
        let id_map = build_snake_id_map(&game);

        let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        EvalMinimaxSnake::new(game, game_info, turn, &score, "devious-devin")
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
            color: Some("#99cc00".to_owned()),
            head: Some("snail".to_owned()),
            tail: Some("rbc-necktie".to_owned()),
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
