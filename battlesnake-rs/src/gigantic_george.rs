use crate::compact_a_prime::NeighborDeterminableGame;

use crate::eremetic_eric::EremeticEric;

use super::*;

pub struct GiganticGeorge {}

fn path_to_full_board(reversed_body: &[Position], game: &Game) -> Option<Vec<(Move, Position)>> {
    let max_size = game.board.width * game.board.height;
    if reversed_body.len() == max_size as usize {
        return Some(vec![]);
    }

    for (dir, coor) in game
        .possible_moves(reversed_body.last().unwrap())
        .iter()
        .filter(|(_, c)| !reversed_body.contains(c))
    {
        let mut new_body = reversed_body.to_vec();
        new_body.push(*coor);

        if let Some(mut path) = path_to_full_board(&new_body, game) {
            path.push((*dir, *coor));
            return Some(path);
        }
    }

    None
}

trait FullBoardDeterminable {
    fn is_full(&self) -> bool;
}

impl FullBoardDeterminable for Game {
    fn is_full(&self) -> bool {
        todo!()
    }
}

impl<T> BattlesnakeAI<T> for GiganticGeorge {
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

    fn make_move(&self, state: T) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(s) = &state.you.shout {
            if s.starts_with("PATH:") {
                let path = s.split("PATH:").nth(1).unwrap();

                let next_char = path.to_lowercase().chars().last().unwrap();
                let dir = match next_char {
                    'l' => Some(Move::Left),
                    'r' => Some(Move::Right),
                    'u' => Some(Move::Up),
                    'd' => Some(Move::Down),
                    _ => None,
                };

                if let Some(d) = dir {
                    return Ok(MoveOutput {
                        r#move: format!("{}", d),
                        shout: Some(path[..path.len() - 2].to_string()),
                    });
                }
            }
        }

        if state.is_full() {
            println!("Ok now can we complete the board?");

            let reversed_body = {
                let mut x = Vec::from(state.you.body.clone());
                x.pop(); // Remove my current tail cause I will need to fill that space too
                x.reverse();
                x
            };

            if let Some(mut path) = path_to_full_board(&reversed_body, &state) {
                println!("Yup lets go that way");
                let new = path.pop();
                let path_string: String = path
                    .iter()
                    .map(|(d, _)| format!("{}", d).chars().next().unwrap())
                    .collect();
                return Ok(MoveOutput {
                    r#move: format!("{}", new.unwrap().0),
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
