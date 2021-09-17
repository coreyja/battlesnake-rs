use battlesnake_game_types::types::{
    FoodGettableGame, NeighborDeterminableGame, RandomReasonableMovesGame, SizeDeterminableGame,
    SnakeBodyGettableGame, YouDeterminableGame,
};

use crate::a_prime::APrimeNextDirection;

use super::*;

pub struct FamishedFrank {}

impl<
        T: SizeDeterminableGame
            + FoodGettableGame
            + PositionGettableGame
            + SnakeBodyGettableGame
            + APrimeNextDirection
            + RandomReasonableMovesGame
            + SnakeIDGettableGame
            + YouDeterminableGame,
    > BattlesnakeAI<T> for FamishedFrank
{
    fn name(&self) -> String {
        "famished-frank".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#FFBB33".to_owned()),
            ..Default::default()
        }
    }

    fn make_move(&self, state: T) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let target_length = state.get_height() * 2 + state.get_width();
        let you_body = state.get_snake_body_vec(state.you_id());
        let targets = if you_body.len() < target_length as usize {
            state.get_all_food_as_native_positions()
        } else {
            vec![
                Position { x: 0, y: 0 },
                Position {
                    x: (state.get_width() - 1) as i32,
                    y: 0,
                },
                Position {
                    x: 0,
                    y: (state.get_height() - 1) as i32,
                },
                Position {
                    x: (state.get_width() - 1) as i32,
                    y: (state.get_height() - 1) as i32,
                },
            ]
            .iter()
            .map(|c| state.native_from_position(*c))
            .collect()
        };

        let targets: Vec<_> = targets
            .into_iter()
            .filter(|t| !you_body.contains(t))
            .collect();

        let head = you_body.first().unwrap();
        let dir = state.shortest_path_next_direction(&head, &targets, None);

        let dir = if let Some(s) = dir {
            s
        } else {
            let you_id = state.you_id();
            state
                .shortest_path_next_direction(&head, &[you_body.last().unwrap().clone()], None)
                .unwrap_or_else(|| {
                    state
                        .random_reasonable_move_for_each_snake()
                        .into_iter()
                        .find(|(s, _)| s == you_id)
                        .map(|x| x.1)
                        .unwrap_or(Move::Right)
                })
        };

        Ok(MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        })
    }
}
