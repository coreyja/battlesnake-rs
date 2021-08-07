use core::hash::Hash;
use std::convert::TryInto;

use itertools::Itertools;

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
        let body_set: HashSet<_> = state.you.body.iter().collect();
        if body_set.len() != state.you.body.len() {
            println!("Ran into yourself");
        }
    }

    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let body = {
            let mut body = state.you.body.clone();
            let path_to_complete_circle =
                a_prime::shortest_path(&state.board, &body[0], &[*body.last().unwrap()]);
            for c in path_to_complete_circle.into_iter() {
                if !body.contains(&c) {
                    body.push(c);
                }
            }
            body
        };
        let modified_board = {
            let mut b = state.board.clone();
            let mut clone_me = state.you.clone();
            clone_me.body = body.clone();
            b.snakes = vec![clone_me];
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
        let (_, (_, best_cost)) = *food_options
            .iter()
            .min_by_key(|(_, (_, cost))| *cost)
            .unwrap();

        let matching_cost_foods: Vec<_> = food_options
            .iter()
            .cloned()
            .filter(|(_, (_, cost))| cost == &best_cost)
            .collect();

        let cost_to_loop = body.len();

        let matching_food_options: Vec<_> = matching_cost_foods
            .iter()
            .map(|(food, (closest_body_part, _))| {
                let closest_index: usize =
                    body.iter().position(|x| &x == closest_body_part).unwrap();

                let tail_index: usize = if closest_index == 0 {
                    state.you.body.len() - 1
                } else {
                    closest_index - 1
                };
                let would_be_tail = body[tail_index];

                let dist_back_from_food_to_tail =
                    a_prime::shortest_distance(&modified_board, food, &[would_be_tail])
                        .unwrap_or(5000);

                let cost_to_get_to_closest: u64 = if closest_index == 0 {
                    0
                } else {
                    (cost_to_loop - closest_index).try_into().unwrap()
                };
                let best_cost_u64: u64 = best_cost.try_into().unwrap();
                let cost_to_get_to_nearest_food: u64 = cost_to_get_to_closest + best_cost_u64;
                let cost_to_get_food_and_then_get_back_if_at_closest_point: u64 =
                    best_cost_u64 + dist_back_from_food_to_tail as u64;

                let health: u64 = state.you.health.try_into().unwrap();
                if (**food == Coordinate { x: 6, y: 5 }) {
                    dbg!(
                        health,
                        cost_to_get_to_closest,
                        cost_to_get_food_and_then_get_back_if_at_closest_point
                    );
                }
                let health_cost: i64 = if health >= cost_to_get_to_nearest_food {
                    ((health - cost_to_get_to_closest)
                        + cost_to_get_food_and_then_get_back_if_at_closest_point)
                        .try_into()
                        .unwrap()
                } else {
                    6666
                };

                ((food, (closest_body_part, best_cost)), (health_cost, food))
            })
            .sorted_by_key(|(_, cost)| *cost)
            .collect();

        let (&best_food, (&closest_body_part, best_cost)) = matching_food_options[0].0;

        let health: u64 = state.you.health.try_into()?;
        let best_cost: u64 = best_cost.try_into()?;
        let cant_survive_another_loop =
            health < TryInto::<u64>::try_into(cost_to_loop)? + best_cost;

        if &state.you.head == closest_body_part && cant_survive_another_loop {
            let d =
                a_prime::shortest_path_next_direction(&state.board, &state.you.head, &[*best_food])
                    .unwrap();

            return Ok(MoveOutput {
                r#move: d.value(),
                shout: None,
            });
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
            && !empty_tail_neighbors.is_empty())
            || !has_unique_elements(state.you.body.iter())
        {
            a_prime::shortest_path_next_direction(
                &state.board,
                &state.you.head,
                &empty_tail_neighbors,
            )
        } else {
            None
        };

        let dir = empty_dir.unwrap_or_else(|| {
            a_prime::shortest_path_next_direction(
                &state.board,
                &state.you.head,
                &[state.you.tail()],
            )
            .unwrap_or(Direction::Up)
        });

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
