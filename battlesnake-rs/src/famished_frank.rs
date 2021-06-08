use super::*;

pub struct FamishedFrank {}

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
        let dir =
            a_prime::shortest_path_next_direction(&state.board, &state.you.head, &state.board.food)
                .unwrap_or(Direction::UP);

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }
}
