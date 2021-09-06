use battlesnake_game_types::types::YouDeterminableGame;

use super::*;

pub struct BombasticBob;

impl BattlesnakeAI for BombasticBob {
    fn make_move(
        &self,
        state: Game,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let chosen = state
            .random_reasonable_move_for_each_snake()
            .into_iter()
            .find(|(s, _)| s == state.you_id())
            .map(|x| x.1);
        let dir = chosen.unwrap_or(Move::Right);

        Ok(MoveOutput {
            r#move: format!("{}", dir),
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
