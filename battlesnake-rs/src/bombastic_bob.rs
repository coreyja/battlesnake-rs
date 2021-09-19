use battlesnake_game_types::types::{
    RandomReasonableMovesGame, SnakeIDGettableGame, YouDeterminableGame,
};

use super::*;

pub struct BombasticBob<T> {
    game: T,
}

impl<T: RandomReasonableMovesGame + SnakeIDGettableGame + YouDeterminableGame> BattlesnakeAI
    for BombasticBob<T>
{
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let chosen = self
            .game
            .random_reasonable_move_for_each_snake()
            .into_iter()
            .find(|(s, _)| s == self.game.you_id())
            .map(|x| x.1);
        let dir = chosen.unwrap_or(Move::Right);

        Ok(MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        })
    }
}

pub struct BombasticBobFactory;

impl BattlesnakeFactory for BombasticBobFactory {
    fn name(&self) -> String {
        "bombastic-bob".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        Box::new(BombasticBob { game })
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            ..Default::default()
        }
    }
}
