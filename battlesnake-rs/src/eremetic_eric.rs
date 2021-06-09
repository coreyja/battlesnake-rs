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
        let food_direction: Option<Direction> = if state.you.health < 15 || state.turn <= 3 {
            a_prime::shortest_path_next_direction(&state.board, &state.you.head, &state.board.food)
        } else {
            None
        };

        let dir: Direction = if let Some(d) = food_direction {
            d
        } else {
            let d = a_prime::shortest_path_next_direction(
                &state.board,
                &state.you.head,
                &state.you.body[state.you.body.len() - 1..],
            )
            .unwrap_or(Direction::UP);

            if state.you.head.move_in(&d) == state.you.body[state.you.body.len() - 2] {
                let possible_moves = state
                    .you
                    .head
                    .possible_moves(&state.board)
                    .iter()
                    .filter(|(_dir, coor)| !state.you.body.contains(coor))
                    .cloned()
                    .collect::<Vec<_>>();
                let chosen = possible_moves.choose(&mut rand::thread_rng());
                chosen.map(|x| x.0).unwrap_or(Direction::UP)
            } else {
                d
            }
        };

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }
}
