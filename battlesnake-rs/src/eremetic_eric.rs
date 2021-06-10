use std::convert::TryInto;

use super::*;

use rand::seq::SliceRandom;

pub struct EremeticEric {}

impl BattlesnakeAI for EremeticEric {
    fn name(&self) -> String {
        "eremetic-eric".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#FF4444".to_owned()),
            ..Default::default()
        }
    }

    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let (best_food, (closest_body_part, cost)) = state
            .board
            .food
            .iter()
            .map(|food| {
                (
                    food,
                    state
                        .you
                        .body
                        .iter()
                        .map(|body_part| (body_part, food.dist_from(body_part)))
                        .min_by_key(|x| x.1)
                        .unwrap(),
                )
            })
            .min_by_key(|(_, (_, cost))| cost.clone())
            .unwrap();

        let health: u64 = state.you.health.try_into().unwrap();
        let cost: u64 = cost.try_into().unwrap();
        if !closest_body_part.on_wall(&state.board)
            && &state.you.head == closest_body_part
            && health < state.you.length + cost
        {
            let d =
                a_prime::shortest_path_next_direction(&state.board, &state.you.head, &[*best_food])
                    .unwrap();

            return Ok(MoveOutput {
                r#move: d.value(),
                shout: None,
            });
        }

        if closest_body_part.on_wall(&state.board) && health < state.you.length + cost {
            let closest_index: usize = state
                .you
                .body
                .iter()
                .position(|x| x == closest_body_part)
                .unwrap()
                .try_into()
                .unwrap();

            let before_index: usize = if closest_index == 0 {
                state.you.body.len() - 1
            } else {
                closest_index - 1
            };
            let before = state.you.body[before_index];

            if !before.on_wall(&state.board) && state.you.head == before {
                let d = a_prime::shortest_path_next_direction(
                    &state.board,
                    &state.you.head,
                    &[*best_food],
                )
                .unwrap();

                return Ok(MoveOutput {
                    r#move: d.value(),
                    shout: None,
                });
            }

            if &state.you.head == closest_body_part {
                let d = a_prime::shortest_path_next_direction(
                    &state.board,
                    &state.you.head,
                    &[*best_food],
                )
                .unwrap();

                return Ok(MoveOutput {
                    r#move: d.value(),
                    shout: None,
                });
            }
        }

        if state.turn < 3 {
            return Ok(MoveOutput {
                r#move: a_prime::shortest_path_next_direction(
                    &state.board,
                    &state.you.head,
                    &state.board.food,
                )
                .unwrap()
                .value(),
                shout: None,
            });
        }

        let tail_dir = a_prime::shortest_path_next_direction(
            &state.board,
            &state.you.head,
            &state.you.body[state.you.body.len() - 1..],
        )
        .unwrap_or(Direction::UP);

        Ok(MoveOutput {
            r#move: tail_dir.value(),
            shout: None,
        })
    }
}
