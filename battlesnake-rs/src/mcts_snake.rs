use battlesnake_game_types::compact_representation::WrappedCellBoard4Snakes11x11;
use tracing::info;

use super::*;

pub struct MctsSnake<T> {
    game: T,
}

impl<T> MctsSnake<T> {
    pub fn new(game: T) -> Self {
        Self { game }
    }
}

impl<T> BattlesnakeAI for MctsSnake<T> {
    fn make_move(&self) -> Result<MoveOutput> {
        let _ = &self.game;
        Ok(MoveOutput {
            r#move: format!("{}", Move::Right),
            shout: None,
        })
    }

    fn end(&self) {
        info!("Mcts has ended");
    }
}

pub struct MctsSnakeFactory;

impl BattlesnakeFactory for MctsSnakeFactory {
    fn name(&self) -> String {
        "mcts".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let id_map = build_snake_id_map(&game);

        if game_info.ruleset.name == "wrapped" {
            let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(game);

            Box::new(snake)
        } else {
            let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(game);

            Box::new(snake)
        }
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
