use std::collections::VecDeque;

use super::*;

pub trait MoveToAndSpawn: NeighborDeterminableGame + PositionGettableGame {
    fn move_to_and_opponent_sprawl(&self, coor: &Self::NativePositionType) -> Self;
}

use battlesnake_game_types::types::{
    HeadGettableGame, HealthGettableGame, NeighborDeterminableGame, PositionGettableGame,
    YouDeterminableGame,
};
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

fn score<
    T: NeighborDeterminableGame + YouDeterminableGame + HealthGettableGame + MoveToAndSpawn,
>(
    game_state: &T,
    coor: &T::NativePositionType,
    times_to_recurse: u8,
) -> i64 {
    const PREFERRED_HEALTH: i64 = 80;
    let you_id = game_state.you_id();

    if game_state.position_is_snake_body(coor.clone()) {
        return 0;
    }

    if !game_state.is_alive(you_id) {
        return 0;
    }

    let ihealth = game_state.get_health_i64(you_id);
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

pub struct AmphibiousArthur<T> {
    game: T,
}

impl<
        T: NeighborDeterminableGame
            + SnakeIDGettableGame
            + HeadGettableGame
            + YouDeterminableGame
            + MoveToAndSpawn
            + HealthGettableGame,
    > BattlesnakeAI for AmphibiousArthur<T>
{
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let you_id = self.game.you_id();
        let possible = self
            .game
            .possible_moves(&self.game.get_head_as_native_position(you_id));
        let recursion_limit: u8 = match std::env::var("RECURSION_LIMIT").map(|x| x.parse()) {
            Ok(Ok(x)) => x,
            _ => 5,
        };
        let next_move = possible
            .iter()
            .max_by_key(|(_mv, coor)| score(&self.game, coor, recursion_limit));

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
}

pub struct AmphibiousArthurFactory;

impl BattlesnakeFactory for AmphibiousArthurFactory {
    fn name(&self) -> String {
        "amphibious-arthur".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        Box::new(AmphibiousArthur { game })
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
