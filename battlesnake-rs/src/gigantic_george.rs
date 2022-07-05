use std::{collections::HashSet, convert::TryInto};

use types::types::*;

use crate::eremetic_eric::EremeticEric;

use super::*;

pub struct GiganticGeorge<T> {
    game: T,
}

fn path_to_full_board<T: PositionGettableGame + SizeDeterminableGame + NeighborDeterminableGame>(
    reversed_body: &[T::NativePositionType],
    game: &T,
) -> Option<Vec<(Move, T::NativePositionType)>> {
    let max_size = game.get_width() * game.get_height();
    if reversed_body.len() == max_size as usize {
        return Some(vec![]);
    }

    for (dir, coor) in game
        .possible_moves(reversed_body.last().unwrap())
        .filter(|(_, c)| !reversed_body.contains(c))
    {
        let mut new_body = reversed_body.to_vec();
        new_body.push(coor.clone());

        if let Some(mut path) = path_to_full_board(&new_body, game) {
            path.push((dir, coor));
            return Some(path);
        }
    }

    None
}

pub trait FullBoardDeterminable {
    fn contains_empty_squares(&self) -> bool;
}
impl FullBoardDeterminable for Game {
    fn contains_empty_squares(&self) -> bool {
        let mut map: HashSet<Position> = HashSet::new();

        for c in self.board.food.iter() {
            map.insert(*c);
        }

        for s in self.board.snakes.iter() {
            for c in s.body.iter() {
                map.insert(*c);
            }
        }

        let full_size: usize = (self.get_height() * self.get_width()).try_into().unwrap();

        full_size != map.len()
    }
}

impl<T> BattlesnakeAI for GiganticGeorge<T>
where
    T: FullBoardDeterminable
        + ShoutGettableGame
        + YouDeterminableGame
        + NeighborDeterminableGame
        + SizeDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + SnakeBodyGettableGame
        + SnakeTailPushableGame
        + types::types::FoodGettableGame
        + types::types::HealthGettableGame
        + a_prime::APrimeNextDirection
        + TurnDeterminableGame
        + std::clone::Clone,
{
    fn make_move(&self) -> Result<MoveOutput> {
        let you_id = self.game.you_id();

        if let Some(s) = self.game.get_shout(you_id) {
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
                        shout: Some(format!("PATH:{}", &path[..path.len() - 1])),
                    });
                }
            }
        }

        if !self.game.contains_empty_squares() {
            println!("Ok now can we complete the board?");

            let reversed_body = {
                let mut x = self.game.get_snake_body_vec(you_id);
                x.pop(); // Remove my current tail cause I will need to fill that space too
                x.reverse();
                x
            };

            if let Some(mut path) = path_to_full_board(&reversed_body, &self.game) {
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

        let eric = EremeticEric {
            game: self.game.clone(),
        };
        eric.make_move()
    }
}

pub struct GiganticGeorgeFactory {}

impl BattlesnakeFactory for GiganticGeorgeFactory {
    fn create_from_wire_game(&self, game: Game) -> BoxedSnake {
        Box::new(GiganticGeorge { game })
    }

    fn name(&self) -> String {
        "gigantic-george".to_owned()
    }
    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#FFBB33".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            ..Default::default()
        }
    }
}
