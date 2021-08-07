use crate::eremetic_eric::EremeticEric;

use super::*;

pub struct GiganticGeorge {}

fn path_to_full_board(
    reversed_body: &[Coordinate],
    board: &Board,
) -> Option<Vec<(Direction, Coordinate)>> {
    let max_size = board.width * board.height;
    if reversed_body.len() == max_size as usize {
        return Some(vec![]);
    }

    for (dir, coor) in reversed_body
        .last()
        .unwrap()
        .possible_moves(board)
        .filter(|(_, c)| !reversed_body.contains(c))
    {
        let mut new_body = reversed_body.to_vec();
        new_body.push(coor);

        if let Some(mut path) = path_to_full_board(&new_body, board) {
            path.push((dir, coor));
            return Some(path);
        }
    }

    None
}

impl BattlesnakeAI for GiganticGeorge {
    fn name(&self) -> String {
        "gigantic-george".to_owned()
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
        let grid = state.board.to_grid();

        if let Some(s) = &state.you.shout {
            if s.starts_with("PATH:") {
                let path = s.split("PATH:").nth(1).unwrap();

                let next_char = path.to_lowercase().chars().last().unwrap();
                let dir = match next_char {
                    'l' => Some(Direction::Left),
                    'r' => Some(Direction::Right),
                    'u' => Some(Direction::Up),
                    'd' => Some(Direction::Down),
                    _ => None,
                };

                if let Some(d) = dir {
                    return Ok(MoveOutput {
                        r#move: d.value(),
                        shout: Some(path[..path.len() - 2].to_string()),
                    });
                }
            }
        }

        if grid.is_full() {
            println!("Ok now can we complete the board?");

            let reversed_body = {
                let mut x = state.you.body.clone();
                x.pop(); // Remove my current tail cause I will need to fill that space too
                x.reverse();
                x
            };
            if let Some(mut path) = path_to_full_board(&reversed_body, &state.board) {
                println!("Yup lets go that way");
                let new = path.pop();
                let path_string: String = path
                    .iter()
                    .map(|(d, _)| d.value().chars().next().unwrap())
                    .collect();
                return Ok(MoveOutput {
                    r#move: new.unwrap().0.value(),
                    shout: Some("PATH:".to_string() + &path_string),
                });
            } else {
                println!("Nah lets keep looping");
            }
        }

        let eric = EremeticEric {};
        eric.make_move(state)
    }
}
