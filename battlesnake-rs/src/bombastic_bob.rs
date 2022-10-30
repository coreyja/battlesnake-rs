use rand::thread_rng;
use types::types::{RandomReasonableMovesGame, SnakeIDGettableGame, YouDeterminableGame};

use super::*;

pub struct BombasticBob<T> {
    game: T,
}

impl<T: RandomReasonableMovesGame + SnakeIDGettableGame + YouDeterminableGame> BattlesnakeAI
    for BombasticBob<T>
{
    type State = ();

    fn make_move(&self, _: Option<Self::State>) -> Result<MoveOutput> {
        let mut rng = thread_rng();
        let chosen = self
            .game
            .random_reasonable_move_for_each_snake(&mut rng)
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
    type Snake = BombasticBob<Game>;

    fn name(&self) -> String {
        "bombastic-bob".to_owned()
    }

    fn create_from_wire_game(&self, game: Game) -> Self::Snake {
        BombasticBob { game }
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
