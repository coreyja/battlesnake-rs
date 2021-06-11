use super::*;

pub struct BombasticBob;

impl BattlesnakeAI for BombasticBob {
    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let chosen = state.you.random_possible_move(&state.board);
        let dir = chosen.map(|x| x.0).unwrap_or(Direction::RIGHT);

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
