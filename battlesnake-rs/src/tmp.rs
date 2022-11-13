
use std::marker::PhantomData;

use types::{types::*, wire_representation};

use crate::MoveOutput;
use anyhow::Result;

trait FactoryTrait {
    type Snake: SnakeTrait;

    fn create_snake(wire_game: wire_representation::Game) -> Self::Snake;
}

trait SnakeTrait {
    fn make_move(&self, game: wire_representation::Game) -> Result<MoveOutput>;
}

struct HobbsFactory<BoardType>(PhantomData<BoardType>);

impl<BoardType> FactoryTrait for HobbsFactory<BoardType>
where
    BoardType: PositionGettableGame,
    BoardType: From<wire_representation::Game>,
{
    type Snake = Hobbs<BoardType>;

    fn create_snake(wire_game: wire_representation::Game) -> Self::Snake {
        Hobbs {
            initial_game: wire_game.into(),
        }
    }
}

struct Hobbs<BoardType: PositionGettableGame> {
    initial_game: BoardType,
}

impl<BoardType> SnakeTrait for Hobbs<BoardType>
where
    BoardType: PositionGettableGame,
    BoardType: From<wire_representation::Game>,
{
    fn make_move(&self, game: wire_representation::Game) -> Result<MoveOutput> {
        todo!();
        let board = BoardType::from(game);

        let move_ = board.get_move();
        Ok(MoveOutput {
            r#move: format!("{}", move_),
            shout: None,
        })
    }
}
