use std::{collections::HashMap, fmt::Debug};

use battlesnake_minimax::paranoid::{MinMaxReturn, MinimaxSnake, WrappedScore};
use color_eyre::eyre::Result;
use itertools::Itertools;
use serde_json::Value;
use types::{
    compact_representation::{dimensions::Square, WrappedCellBoard},
    types::{build_snake_id_map, Move, SnakeIDGettableGame, SnakeId, YouDeterminableGame},
};

use crate::unofficial_api::{frame_to_game, get_frame_for_turn};

#[derive(clap::Args, Debug)]
pub(crate) struct Solve {
    /// Game ID to debug
    #[clap(short, long, value_parser)]
    game_id: String,

    /// Number of times to greet
    #[clap(short, long, value_parser)]
    you_name: String,

    /// Number of turns past the last frame to check
    #[clap(short, long, value_parser, default_value_t = 20)]
    turns_after_lose: i32,

    /// Turn to start looking back from. Uses the last turn of the game if not specified
    #[clap(short, long, value_parser)]
    search_starting_turn: Option<i32>,
}

impl Solve {
    pub(crate) fn run(self) -> Result<()> {
        let body: Value =
            ureq::get(format!("https://engine.battlesnake.com/games/{}", self.game_id).as_str())
                .call()?
                .into_json()?;

        let last_frame = &body["LastFrame"];
        let last_turn = last_frame["Turn"].as_i64().expect("Missing Turn") as i32;
        let mut current_turn = self.search_starting_turn.unwrap_or(last_turn - 1);

        loop {
            let current_frame = get_frame_for_turn(&self.game_id, current_turn)?;
            let wire_game = frame_to_game(&current_frame, &body["Game"], &self.you_name);

            if wire_game.is_ok() {
                break;
            }
            println!("You were not alive at turn {current_turn} moving backwards");

            current_turn -= 1;

            if current_turn < 0 {
                panic!("Something is wrong we made it past the end of the game");
            }
        }

        let last_living_turn = current_turn;

        println!("Ending Turn {}", &last_frame["Turn"]);
        println!("Last Living Turn {last_living_turn}");

        loop {
            let current_frame = get_frame_for_turn(&self.game_id, current_turn)?;
            let wire_game = frame_to_game(&current_frame, &body["Game"], &self.you_name).unwrap();

            if !wire_game.is_wrapped() {
                unimplemented!("Only implementing for wrapped games, RIGHT NOW");
            }

            let snake_ids = build_snake_id_map(&wire_game);
            let game_info = wire_game.game.clone();
            let game: WrappedCellBoard<u16, Square, { 11 * 11 }, 8> =
                wire_game.as_wrapped_cell_board(&snake_ids).unwrap();

            let you_id = game.you_id();

            let explorer_snake =
                MinimaxSnake::from_fn(game, game_info, current_turn, &|_| {}, "explorer");

            let max_turns = (last_living_turn + 1 - current_turn + self.turns_after_lose) as usize;
            let result = explorer_snake.deepend_minimax_to_turn(max_turns);

            let score = *result.score();

            if matches!(score, WrappedScore::Lose(..) | WrappedScore::Tie(..)) {
                println!("At turn {current_turn}, there were no safe options");
            } else if matches!(score, WrappedScore::Win(_)) {
                println!("At turn {current_turn}, you could have won!");
                if let MinMaxReturn::Node { options, .. } = &result {
                    let winning_moves = options
                        .iter()
                        .filter(|(_, r)| matches!(r.score(), WrappedScore::Win(_)))
                        .map(|(m, _)| *m)
                        .collect_vec();

                    println!("At turn {current_turn}, the winning moves were {winning_moves:?}",);
                    print_moves(&result, current_turn, winning_moves[0]);
                }
                break;
            } else if let MinMaxReturn::Node {
                options,
                moving_snake_id,
                ..
            } = &result
            {
                assert!(moving_snake_id == you_id);
                let safe_options = options
                    .iter()
                    .filter(|(_, r)| matches!(r.score(), WrappedScore::Scored(_)))
                    .collect_vec();
                let safe_moves = safe_options.iter().map(|(m, _)| *m).collect_vec();

                println!("At turn {current_turn}, the safe options were {safe_moves:?}",);
                println!("Turn {current_turn} is the decision point");

                for m in safe_moves {
                    print_moves(&result, current_turn, m);
                }

                // let mut file = File::create("tmp.dot").unwrap();
                // file.write_all(format!("{}", result.to_dot_graph(you_id)).as_bytes())
                //     .unwrap();

                // Command::new("dot")
                //     .arg("-Tsvg")
                //     .arg("-O")
                //     .arg("tmp.dot")
                //     .output()
                //     .unwrap();
                // Command::new("open").arg("tmp.dot.svg").output().unwrap();

                break;
            } else {
                panic!("We shouldn't ever have a leaf here")
            }

            current_turn -= 1;
        }

        Ok(())
    }
}

fn print_moves<GameType, ScoreType>(
    result: &MinMaxReturn<GameType, ScoreType>,
    current_turn: i32,
    m: Move,
) where
    GameType: SnakeIDGettableGame<SnakeIDType = SnakeId> + Debug + Clone,
    ScoreType: Copy + Ord + Debug,
{
    let all_snake_path = result.chosen_route();
    let sids = all_snake_path
        .iter()
        .map(|(sid, _)| sid)
        .unique()
        .collect_vec();
    let mut paths_per_snake: HashMap<SnakeId, Vec<Move>> = HashMap::new();
    for &sid in &sids {
        let path = all_snake_path
            .iter()
            .filter(|(s, _)| s == sid)
            .map(|(_, p)| p)
            .cloned()
            .collect_vec();
        paths_per_snake.insert(*sid, path);
    }
    println!(
        "At turn {current_turn}, the {m} path takes {} turn lookahead:",
        all_snake_path.len() / sids.len()
    );
    for (sid, path) in paths_per_snake {
        println!("{sid:?}: {}", path.iter().join(", "));
    }
    println!()
}
