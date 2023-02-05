use std::collections::VecDeque;

use battlesnake_rs::SizeDeterminableGame;
use itertools::Itertools;
use rand::{
    prelude::{IteratorRandom, ThreadRng},
    thread_rng, Rng,
};
use types::{
    types::*,
    wire_representation::{BattleSnake, Board, Game, NestedGame, Position, Ruleset},
};

fn random_square_for_head(rng: &mut ThreadRng, g: &Game) -> Option<Position> {
    let width_range = (0..g.get_width()).collect_vec();
    let height_range = (0..g.get_height()).collect_vec();
    let ranges = [width_range, height_range];
    let multi = ranges.iter().multi_cartesian_product();
    multi
        .map(|pos| Position {
            x: *pos[0] as i32,
            y: *pos[1] as i32,
        })
        .filter(|p| !g.position_is_snake_body(*p))
        .choose(rng)
}

fn random_snake(rng: &mut ThreadRng, id: &str, g: &Game) -> Option<BattleSnake> {
    let health = rng.gen_range(1..=100);
    let length: i32 = rng.gen_range(3..20);

    let head = random_square_for_head(rng, g)?;

    let mut body: VecDeque<Position> = VecDeque::with_capacity(length as usize);
    body.push_front(head);

    while body.len() < length as usize {
        if let Some(next_body) = g
            .neighbors(body.back().unwrap())
            .into_iter()
            .filter(|p| !body.contains(p) && !g.position_is_snake_body(*p))
            .choose(rng)
        {
            body.push_back(next_body);
        } else {
            break;
        }
    }

    if body.len() < 3 {
        return None;
    }

    Some(BattleSnake {
        id: id.to_owned(),
        name: id.to_owned(),
        health,
        actual_length: Some(length),
        shout: None,
        head,
        body,
    })
}

pub fn random_game() -> Game {
    let mut rng = thread_rng();

    let nested_game = NestedGame {
        timeout: 600,
        id: "faked".to_owned(),
        ruleset: Ruleset {
            name: "standard".to_owned(),
            version: "1".to_owned(),
            settings: None,
        },
        map: None,
        source: None,
    };

    // TODO: Choose width and height randomly too
    let mut game = Game {
        game: nested_game,
        turn: 0,
        board: Board {
            width: 11,
            height: 11,
            food: vec![],
            hazards: vec![],
            snakes: vec![],
        },
        you: BattleSnake {
            id: "".to_owned(),
            body: VecDeque::new(),
            actual_length: None,
            health: 0,
            name: "".to_owned(),
            shout: None,
            head: Position { x: 0, y: 0 },
        },
    };

    // TODO: Maybe choose more snakes if the board is bigger?
    let number_of_snakes: i8 = rng.gen_range(2..=2);

    for i in 0..number_of_snakes {
        // TODO: Would it be better to build all snakes at once? Not sure it matters if we run
        // enough games... But would it make the games more realistic? If so that would reduce the
        // time of fuzzing which would be good
        if let Some(s) = random_snake(&mut rng, &format!("{}", i), &game) {
            game.board.snakes.push(s);
        } else {
            break;
        }
    }

    // TODO: Choose a max number of foods and try to add them to the board
    // TODO: Choose a max number of hazards squares and add them to the board

    if let Some(you) = game.board.snakes.get(0) {
        game.you = you.clone();
    }

    game
}
