use tracing::info;

use super::*;

pub struct ConstantCarter {}

impl BattlesnakeAI for ConstantCarter {
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        Ok(MoveOutput {
            r#move: format!("{}", Move::Right),
            shout: None,
        })
    }

    fn end(&self) {
        info!("ConstantCarter has ended");
    }
}

pub struct ConstantCarterFactory;

impl BattlesnakeFactory for ConstantCarterFactory {
    fn name(&self) -> String {
        "constant-carter".to_owned()
    }

    fn from_wire_game(&self, _game: Game) -> BoxedSnake {
        Box::new(ConstantCarter {})
    }
    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            ..Default::default()
        }
    }
}
