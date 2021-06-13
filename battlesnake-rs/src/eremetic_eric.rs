use std::convert::TryInto;

use super::*;

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
        let food_options: Vec<_> = state
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
            .collect();
        let (_, (_, best_cost)) = food_options
            .iter()
            .min_by_key(|(_, (_, cost))| cost.clone())
            .unwrap()
            .clone();

        let matching_cost_foods: Vec<_> = food_options
            .iter()
            .cloned()
            .filter(|(_, (_, cost))| cost == &best_cost)
            .collect();

        let (best_food, (closest_body_part, best_cost)) = matching_cost_foods
            .iter()
            .min_by_key(|(food, (closest_body_part, best_cost))| {
                let closest_index: usize = state
                    .you
                    .body
                    .iter()
                    .position(|x| &x == closest_body_part)
                    .unwrap();

                let tail_index: usize = if closest_index == 0 {
                    state.you.body.len() - 1
                } else {
                    closest_index - 1
                };
                let would_be_tail = state.you.body[tail_index];

                (
                    a_prime::shortest_distance(&state.board, food, &[would_be_tail])
                        .unwrap_or(i64::MAX),
                    food,
                )
            })
            .unwrap()
            .clone();

        let health: u64 = state.you.health.try_into()?;
        let best_cost: u64 = best_cost.try_into()?;
        let cost_to_loop: u64 =
            state.you.length + state.you.head.dist_from(&state.you.tail()) as u64;
        let cant_survive_another_loop = health < cost_to_loop + best_cost;
        dbg!(best_food, cant_survive_another_loop);

        if !closest_body_part.on_wall(&state.board)
            && &state.you.head == closest_body_part
            && cant_survive_another_loop
        {
            let d =
                a_prime::shortest_path_next_direction(&state.board, &state.you.head, &[*best_food])
                    .unwrap();

            return Ok(MoveOutput {
                r#move: d.value(),
                shout: None,
            });
        }

        if closest_body_part.on_wall(&state.board) && cant_survive_another_loop {
            let closest_index: usize = state
                .you
                .body
                .iter()
                .position(|x| x == closest_body_part)
                .unwrap()
                .try_into()?;

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

        let empty = state.board.empty_coordiates();
        let tail = state.you.body[state.you.body.len() - 1];
        let empty_tail_neighbors: Vec<_> = tail
            .possible_moves(&state.board)
            .into_iter()
            .map(|x| x.1)
            .filter(|x| empty.contains(x))
            .collect();

        let empty_dir = if state.board.filled_coordinates().len() as f64
            >= (state.board.width * state.board.height) as f64 * 0.95
            && empty_tail_neighbors.len() > 0
        {
            a_prime::shortest_path_next_direction(
                &state.board,
                &state.you.head,
                &empty_tail_neighbors,
            )
        } else {
            None
        };

        let tail_dir =
            a_prime::shortest_path_next_direction(&state.board, &state.you.head, &[tail])
                .unwrap_or(Direction::UP);

        let dir = empty_dir.unwrap_or(tail_dir);

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }
}
