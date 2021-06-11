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
        let target_length = state.board.height * 2 + state.board.width;
        let targets = if state.you.length < target_length.into() {
            state.board.food.clone()
        } else {
            vec![
                Coordinate { x: 0, y: 0 },
                Coordinate {
                    x: (state.board.width - 1).into(),
                    y: 0,
                },
                Coordinate {
                    x: 0,
                    y: (state.board.height - 1).into(),
                },
                Coordinate {
                    x: (state.board.width - 1).into(),
                    y: (state.board.height - 1).into(),
                },
            ]
        };

        let targets: Vec<_> = targets
            .into_iter()
            .filter(|t| !state.you.body.contains(t))
            .collect();

        let dir = a_prime::shortest_path_next_direction(&state.board, &state.you.head, &targets);

        let dir = if let Some(s) = dir {
            s
        } else {
            a_prime::shortest_path_next_direction(
                &state.board,
                &state.you.head,
                &state.you.body[state.you.body.len() - 1..],
            )
            .unwrap_or(
                state
                    .you
                    .random_possible_move(&state.board)
                    .map(|x| x.0)
                    .unwrap_or(Direction::RIGHT),
            )
        };

        Ok(MoveOutput {
            r#move: dir.value(),
            shout: None,
        })
    }
}
