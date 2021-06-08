use super::*;

pub struct FamishedFrank {}

fn direction_from_coordinate(from: &Coordinate, to: &Coordinate) -> Option<Direction> {
    if from.x == to.x && from.y + 1 == to.y {
        Some(Direction::UP)
    } else if from.x == to.x && from.y - 1 == to.y {
        Some(Direction::DOWN)
    } else if from.x - 1 == to.x && from.y == to.y {
        Some(Direction::LEFT)
    } else if from.x + 1 == to.x && from.y == to.y {
        Some(Direction::RIGHT)
    } else {
        None
    }
}

impl BattlesnakeAI for FamishedFrank {
    fn name(&self) -> String {
        "famished-frank".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#FFBB33".to_owned()),
            ..Default::default()
        }
    }

    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let shortest_path =
            a_prime::shortest_path(&state.board, &state.you.head, &state.board.food);
        let next_coordinate = shortest_path.get(1);

        let dir = if let Some(c) = next_coordinate {
            direction_from_coordinate(&state.you.head, &c).unwrap()
        } else {
            Direction::UP
        };

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }
}
