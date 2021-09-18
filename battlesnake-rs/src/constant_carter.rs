use super::*;

pub struct ConstantCarter {}

impl BattlesnakeAI for ConstantCarter {
    fn name(&self) -> String {
        "constant-carter".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            ..Default::default()
        }
    }

    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        Ok(MoveOutput {
            r#move: format!("{}", Move::Right),
            shout: None,
        })
    }

    fn from_wire_game(_game: Game) -> Self {
        Self {}
    }
}
