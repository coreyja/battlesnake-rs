use std::convert::TryInto;

use battlesnake_game_types::types::{
    FoodGettableGame, HeadGettableGame, HealthGettableGame, YouDeterminableGame,
};
use itertools::Itertools;

use crate::{
    compact_a_prime::{
        dist_between, dist_between_new, APrimeCalculable, APrimeNextDirection, APrimeOptions,
    },
    gigantic_george::SnakeBodyGettableGame,
};

use super::*;

pub struct EremeticEric {}

pub trait TurnDeterminableGame {
    fn turn(&self) -> u64;
}

impl<
        T: TurnDeterminableGame
            + SnakeBodyGettableGame
            + YouDeterminableGame
            + APrimeCalculable
            + APrimeNextDirection
            + SnakeTailPushableGame
            + Clone
            + FoodGettableGame
            + HealthGettableGame
            + APrimeNextDirection
            + HeadGettableGame
            + FoodGettableGame,
    > BattlesnakeAI<T> for EremeticEric
{
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

    fn end(&self, state: T) {
        println!("Died at turn: {}", state.turn());
        let you_vec = state.get_snake_body_vec(state.you_id());
        let body_set: HashSet<_> = you_vec.iter().collect();
        if body_set.len() != you_vec.len() {
            println!("Ran into yourself");
        }
    }

    fn make_move(&self, state: T) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let you_id = state.you_id();
        let body = state.get_snake_body_vec(state.you_id());
        let modified_board = {
            let mut b = state.clone();
            let mut path_to_complete_circle =
                state.shortest_path(&body[0], &[body.last().unwrap().clone()], None);
            path_to_complete_circle.reverse();
            for c in path_to_complete_circle.into_iter() {
                if !body.contains(&c) {
                    b.push_tail(you_id, c);
                }
            }

            b
        };
        let all_food = state.get_all_food_as_native_positions();

        let food_options: Vec<_> = all_food
            .iter()
            .map(|food| {
                let body_options = body
                    .iter()
                    .map(|body_part| (body_part, dist_between_new(&state, food, body_part)))
                    .collect_vec();
                let best = body_options.iter().cloned().min_by_key(|x| x.1).unwrap();
                body_options
                    .iter()
                    .filter(|(_, cost)| *cost == best.1)
                    .cloned()
                    .map(|(body, _)| (food, (body, best.1)))
                    .collect_vec()
            })
            .flatten()
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
                    body.len() - 1
                } else {
                    closest_index - 1
                };
                let would_be_tail = body[tail_index].clone();

                let dist_back_from_food_to_tail = {
                    modified_board
                        .shortest_distance(food, &[would_be_tail], None)
                        .unwrap_or(5000)
                };

                let cost_to_get_to_closest: u64 = if closest_index == 0 {
                    0
                } else {
                    (cost_to_loop - closest_index).try_into().unwrap()
                };
                let best_cost_u64: u64 = best_cost.try_into().unwrap();
                let cost_to_get_to_nearest_food: u64 = cost_to_get_to_closest + best_cost_u64;
                let cost_to_get_food_and_then_get_back_if_at_closest_point: u64 =
                    best_cost_u64 + dist_back_from_food_to_tail as u64;

                let health = state.get_health_i64(you_id) as u64;
                let health_cost: i64 = if health >= cost_to_get_to_nearest_food {
                    let health_when_at_closest = health - cost_to_get_to_closest;

                    (health_when_at_closest
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

        let health: u64 = state.get_health_i64(you_id).try_into()?;
        let best_cost: u64 = best_cost.try_into()?;
        let cant_survive_another_loop =
            health < TryInto::<u64>::try_into(cost_to_loop)? + best_cost;
        let you_head = state.get_head_as_native_position(you_id);

        if &you_head == closest_body_part && cant_survive_another_loop {
            let d = state
                .shortest_path_next_direction(&you_head, &[best_food.clone()], None)
                .unwrap();

            return Ok(MoveOutput {
                r#move: format!("{}", d),
                shout: None,
            });
        }

        if state.turn() < 3 {
            return Ok(MoveOutput {
                r#move: format!(
                    "{}",
                    state
                        .shortest_path_next_direction(
                            &you_head,
                            &state.get_all_food_as_native_positions(),
                            None
                        )
                        .unwrap()
                ),
                shout: None,
            });
        }

        let dir = state
            .shortest_path_next_direction(
                &you_head,
                &[state.get_snake_body_vec(you_id).last().unwrap().clone()],
                Some(APrimeOptions { food_penalty: 1 }),
            )
            .unwrap();

        Ok(MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        })
    }
}
