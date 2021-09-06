use battlesnake_game_types::types::YouDeterminableGame;

use crate::compact_a_prime::APrimeNextDirection;

use super::*;

pub struct FamishedFrank {}

impl BattlesnakeAI for FamishedFrank {
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

    fn make_move(
        &self,
        state: Game,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let target_length = state.board.height * 2 + state.board.width;
        let targets = if state.you.body.len() < target_length as usize {
            state.board.food.clone()
        } else {
            vec![
                Position { x: 0, y: 0 },
                Position {
                    x: (state.board.width - 1) as i32,
                    y: 0,
                },
                Position {
                    x: 0,
                    y: (state.board.height - 1) as i32,
                },
                Position {
                    x: (state.board.width - 1) as i32,
                    y: (state.board.height - 1) as i32,
                },
            ]
        };

        let targets: Vec<_> = targets
            .into_iter()
            .filter(|t| !state.you.body.contains(t))
            .collect();

        let dir = state.shortest_path_next_direction(&state.you.head, &targets, None);

        let dir = if let Some(s) = dir {
            s
        } else {
            state
                .shortest_path_next_direction(
                    &state.you.head,
                    &[*state.you.body.back().unwrap()],
                    None,
                )
                .unwrap_or_else(|| {
                    state
                        .random_reasonable_move_for_each_snake()
                        .into_iter()
                        .find(|(s, _)| s == state.you_id())
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
