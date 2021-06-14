use core::hash::Hash;
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

    fn end(&self, state: GameState) {
        println!("Died at turn: {}", state.turn);
    }

    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let body = {
            let mut body = state.you.body.clone();
            let path_to_complete_circle =
                a_prime::shortest_path(&state.board, &body[0], &[body[body.len() - 1]]);
            for c in path_to_complete_circle.into_iter() {
                if state.board.empty_coordiates().contains(&c) {
                    body.push(c);
                }
            }
            body
        };
        let modified_board = {
            let mut b = state.board.clone();
            b.snakes = vec![state.you.clone()];
            b
        };
        let food_options: Vec<_> = state
            .board
            .food
            .iter()
            .map(|food| {
                (
                    food,
                    body.iter()
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
                let closest_index: usize =
                    body.iter().position(|x| &x == closest_body_part).unwrap();

                let tail_index: usize = if closest_index == 0 {
                    body.len() - 1
                } else {
                    closest_index - 1
                };
                let would_be_tail = body[tail_index];

                (
                    a_prime::shortest_distance(&modified_board, food, &[would_be_tail])
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
            let closest_index: usize = body
                .iter()
                .position(|x| x == closest_body_part)
                .unwrap()
                .try_into()?;

            let before_index: usize = if closest_index == 0 {
                body.len() - 1
            } else {
                closest_index - 1
            };
            let before = body[before_index];

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
        let empty_tail_neighbors: Vec<_> = state
            .you
            .tail()
            .possible_moves(&state.board)
            .into_iter()
            .map(|x| x.1)
            .filter(|x| empty.contains(x))
            .collect();

        let empty_dir = if (state.board.filled_coordinates().len() as f64
            >= (state.board.width * state.board.height) as f64 * 0.95
            && empty_tail_neighbors.len() > 0)
            || !has_unique_elements(state.you.body.iter())
        {
            let x = a_prime::shortest_path_next_direction(
                &state.board,
                &state.you.head,
                &empty_tail_neighbors,
            );
            x
        } else {
            None
        };

        let tail_dir = a_prime::shortest_path_next_direction(
            &state.board,
            &state.you.head,
            &[state.you.tail()],
        )
        .unwrap_or(Direction::UP);

        let dir = empty_dir.unwrap_or(tail_dir);

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }
}

fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}
