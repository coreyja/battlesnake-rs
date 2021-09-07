use std::collections::VecDeque;

use crate::compact_a_prime::NeighborDeterminableGame;

use super::*;

trait MoveToAndSpawn: NeighborDeterminableGame {
    fn move_to_and_opponent_sprawl(&self, coor: &Position) -> Self;
}

use rand::seq::SliceRandom;

impl MoveToAndSpawn for Game {
    fn move_to_and_opponent_sprawl(&self, coor: &Position) -> Self {
        let mut cloned = self.clone();
        cloned.move_to(coor, &self.you.id);

        let opponents = cloned
            .board
            .snakes
            .iter_mut()
            .filter(|s| s.id == self.you.id);

        for s in opponents {
            let new_body = self.neighbors(&s.head);
            s.head = *new_body.choose(&mut rand::thread_rng()).unwrap();
            s.body.append(&mut VecDeque::from(new_body));
        }

        cloned
    }
}

fn score(game_state: &Game, coor: &Position, times_to_recurse: u8) -> i64 {
    const PREFERRED_HEALTH: i64 = 80;

    if game_state.you.body.contains(coor) {
        return 0;
    }

    if game_state.you.health == 0 {
        return 0;
    }

    if game_state
        .board
        .snakes
        .iter()
        .any(|x| x.body.contains(coor))
    {
        return 0;
    }

    let ihealth: i64 = game_state.you.health.into();
    let current_score: i64 = (ihealth - PREFERRED_HEALTH).abs();
    let current_score = PREFERRED_HEALTH - current_score;

    if times_to_recurse == 0 {
        return current_score;
    }

    let recursed_score: i64 = game_state
        .neighbors(coor)
        .into_iter()
        .map(|c| {
            score(
                &game_state.move_to_and_opponent_sprawl(coor),
                &c,
                times_to_recurse - 1,
            )
        })
        .sum();

    current_score + recursed_score / 2
}

pub struct AmphibiousArthur {}

impl BattlesnakeAI for AmphibiousArthur {
    fn name(&self) -> String {
        "amphibious-arthur".to_owned()
    }

    fn make_move(
        &self,
        game_state: Game,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let possible = game_state.possible_moves(&game_state.you.head);
        let recursion_limit: u8 = match std::env::var("RECURSION_LIMIT").map(|x| x.parse()) {
            Ok(Ok(x)) => x,
            _ => 5,
        };
        let next_move = possible
            .iter()
            .max_by_key(|(_mv, coor)| score(&game_state, coor, recursion_limit));

        let stuck_response: MoveOutput = MoveOutput {
            r#move: format!("{}", Move::Up),
            shout: Some("Oh NO we are stuck".to_owned()),
        };

        let output = next_move.map_or(stuck_response, |(dir, _coor)| MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        });

        Ok(output)
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            head: Some("chomp".to_owned()),
            tail: Some("swirl".to_owned()),
            version: None,
        }
    }
}
