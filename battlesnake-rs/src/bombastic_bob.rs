use rand::seq::SliceRandom;

use super::*;

pub struct BombasticBob;

impl BattlesnakeAI for BombasticBob {
    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let body_set: HashSet<&Coordinate> = state.you.body.iter().collect();
        let possible_moves = state
            .you
            .head
            .possible_moves(&state.board)
            .iter()
            .filter(|(_dir, coor)| !body_set.contains(coor))
            .cloned()
            .collect::<Vec<_>>();
        let chosen = possible_moves.choose(&mut rand::thread_rng());
        let dir = match chosen {
            Some(x) => x.0,
            _ => Direction::DOWN,
        };

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }

    fn name(&self) -> String {
        "bombastic-bob".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            ..Default::default()
        }
    }
}
