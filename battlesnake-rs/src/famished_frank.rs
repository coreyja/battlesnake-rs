use rand::thread_rng;
use types::types::*;

use crate::a_prime::{APrimeNextDirection, APrimeOptions};

use super::*;

pub struct FamishedFrank<T> {
    game: T,
}

impl<T> BattlesnakeAI for FamishedFrank<T>
where
    T: SizeDeterminableGame
        + FoodGettableGame
        + PositionGettableGame
        + SnakeBodyGettableGame
        + APrimeNextDirection
        + RandomReasonableMovesGame
        + SnakeIDGettableGame
        + YouDeterminableGame,
{
    type State = ();

    fn make_move(&self, _: Option<Self::State>) -> Result<MoveOutput> {
        let target_length = self.game.get_height() * 2 + self.game.get_width();
        let you_body = self.game.get_snake_body_vec(self.game.you_id());
        let targets = if you_body.len() < target_length as usize {
            self.game.get_all_food_as_native_positions()
        } else {
            vec![
                Position { x: 0, y: 0 },
                Position {
                    x: (self.game.get_width() - 1) as i32,
                    y: 0,
                },
                Position {
                    x: 0,
                    y: (self.game.get_height() - 1) as i32,
                },
                Position {
                    x: (self.game.get_width() - 1) as i32,
                    y: (self.game.get_height() - 1) as i32,
                },
            ]
            .iter()
            .map(|c| self.game.native_from_position(*c))
            .collect()
        };

        let targets: Vec<_> = targets
            .into_iter()
            .filter(|t| !you_body.contains(t))
            .collect();

        let head = you_body.first().unwrap();
        let dir = self.game.shortest_path_next_direction(
            head,
            &targets,
            Some(APrimeOptions {
                hazard_penalty: 100,
                ..Default::default()
            }),
        );

        let dir = if let Some(s) = dir {
            s
        } else {
            let you_id = self.game.you_id();
            self.game
                .shortest_path_next_direction(
                    head,
                    &[you_body.last().unwrap().clone()],
                    Some(APrimeOptions {
                        hazard_penalty: 100,
                        ..Default::default()
                    }),
                )
                .unwrap_or_else(|| {
                    let mut rng = thread_rng();
                    let next_move = self
                        .game
                        .random_reasonable_move_for_each_snake(&mut rng)
                        .into_iter()
                        .find(|(s, _)| s == you_id)
                        .map(|x| x.1)
                        .unwrap_or(Move::Right);
                    next_move
                })
        };

        Ok(MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        })
    }
}

pub struct FamishedFrankFactory {}

impl BattlesnakeFactory for FamishedFrankFactory {
    type Snake = FamishedFrank<Game>;

    fn name(&self) -> String {
        "famished-frank".to_owned()
    }

    fn create_from_wire_game(&self, game: Game) -> Self::Snake {
        FamishedFrank { game }
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#FFBB33".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            ..Default::default()
        }
    }
}
