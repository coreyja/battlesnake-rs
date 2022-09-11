use color_eyre::eyre::{eyre, Result, WrapErr};

use std::{collections::HashMap, fmt::Debug, fs::File, io::Write};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use battlesnake_minimax::paranoid::{MinMaxReturn, MinimaxSnake, WrappedScore};
use types::{
    compact_representation::{dimensions::ArcadeMaze, WrappedCellBoard},
    types::{build_snake_id_map, Move, SnakeIDGettableGame, SnakeId, YouDeterminableGame},
    wire_representation::{BattleSnake, Board, Game, NestedGame, Position, Ruleset, Settings},
};

fn frame_to_nested_game(game: &Value) -> Result<NestedGame> {
    let id = game["ID"]
        .as_str()
        .ok_or(eyre!("Missing Game ID"))?
        .to_string();

    let map = game["Map"].as_str().map(|x| x.to_string());
    let source = game["Source"].as_str().map(|x| x.to_string());

    let timeout = game["SnakeTimeout"]
        .as_i64()
        .ok_or(eyre!("Missing Timeout"))?;

    let ruleset_name = game["Ruleset"]["name"]
        .as_str()
        .ok_or(eyre!("Missing Ruleset Name"))?
        .to_string();
    let ruleset_version = "No version in frames".to_string();

    let settings = Settings {
        food_spawn_chance: game["Ruleset"]["foodSpawnChance"]
            .as_str()
            .ok_or(eyre!("Missing Food Spawn Chance"))?
            .parse()
            .wrap_err("Too big for an i32")?,
        minimum_food: game["Ruleset"]["minimumFood"]
            .as_str()
            .ok_or(eyre!("Missing minimumFood"))?
            .parse()
            .wrap_err("Too big for an i32")?,
        hazard_damage_per_turn: game["Ruleset"]["damagePerTurn"]
            .as_str()
            .ok_or(eyre!("Missing damagePerTurn"))?
            .parse()
            .wrap_err("Too big for an i32")?,
        hazard_map: None,
        hazard_map_author: None,
        royale: None,
    };

    let ruleset = Ruleset {
        name: ruleset_name,
        version: ruleset_version,
        settings: Some(settings),
    };

    Ok(NestedGame {
        id,
        map,
        source,
        timeout,
        ruleset,
    })
}

fn value_to_position_vec(value: &Value) -> Result<Vec<Position>> {
    value
        .as_array()
        .ok_or(eyre!("Not an array"))?
        .iter()
        .map(|pos| {
            let x = pos["X"]
                .as_i64()
                .ok_or(eyre!("X is not an integer"))?
                .try_into()
                .wrap_err("Too big for an i32")?;

            let y = pos["Y"]
                .as_i64()
                .ok_or(eyre!("Y is not an integer"))?
                .try_into()
                .wrap_err("Too big for an i32")?;

            Ok(Position { x, y })
        })
        .collect()
}

fn value_to_snake(value: &Value) -> Result<BattleSnake> {
    let id = value["ID"].as_str().ok_or(eyre!("Missing ID"))?.to_string();
    let name = value["Name"]
        .as_str()
        .ok_or(eyre!("Missing Name"))?
        .to_string();
    let body = value_to_position_vec(&value["Body"])?;
    let head = body[0];
    let health = value["Health"]
        .as_i64()
        .ok_or(eyre!("Missing Health"))?
        .try_into()
        .wrap_err("Health is too big for an i32")?;
    let shout = value["Shout"].as_str().map(|x| x.to_string());
    let length = body.len() as i32;

    Ok(BattleSnake {
        id,
        name,
        head,
        body: body.into(),
        health,
        shout,
        actual_length: Some(length),
    })
}

fn frame_to_board(frame: &Value, game: &Value) -> Result<Board> {
    let height = game["Height"]
        .as_i64()
        .ok_or(eyre!("Missing Height"))?
        .try_into()
        .wrap_err("Height doesn't fit in a u32")?;

    let width = game["Width"]
        .as_i64()
        .ok_or(eyre!("Missing Width"))?
        .try_into()
        .wrap_err("Width doesn't fit in a u32")?;

    let snakes = frame["Snakes"]
        .as_array()
        .ok_or(eyre!("Missing Snakes"))?
        .iter()
        .filter(|snake_json| snake_json["Death"].is_null())
        .map(value_to_snake)
        .collect::<Result<Vec<BattleSnake>>>()?;

    Ok(Board {
        height,
        width,
        food: value_to_position_vec(&frame["Food"])?,
        hazards: value_to_position_vec(&frame["Hazards"])?,
        snakes,
    })
}

fn frame_to_game(frame: &Value, game: &Value, you_name: &str) -> Result<Game> {
    let turn = frame["Turn"]
        .as_i64()
        .ok_or(eyre!("Turn is not an integer"))?
        .try_into()
        .wrap_err("Turn is too big for an i32")?;

    let nested_game = frame_to_nested_game(game)?;

    let board = frame_to_board(frame, game)?;

    let you = board
        .snakes
        .iter()
        .find(|snake| snake.name == you_name)
        .unwrap_or_else(|| {
            board
                .snakes
                .first()
                .expect("There are no snakes in this game")
        })
        .clone();

    Ok(Game {
        turn,
        game: nested_game,
        board,
        you,
    })
}

use clap::{Parser, Subcommand};

#[derive(clap::Args, Debug)]
struct Solve {
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

#[derive(clap::Args, Debug)]
struct Fixture {
    /// Game ID to debug
    #[clap(short, long, value_parser)]
    game_id: String,

    /// The name of the snake to use as "you"
    #[clap(short, long, value_parser)]
    you_name: String,

    /// Turn to make a fixture for
    #[clap(short, long, value_parser)]
    turn: i32,
}

#[derive(clap::Args, Debug)]
struct Archive {
    /// Game ID to debug
    #[clap(short, long, value_parser)]
    game_id: String,

    /// The name of the snake to use as "you"
    #[clap(short, long, value_parser)]
    you_name: String,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Adds files to myapp
    Solve(Solve),
    Fixture(Fixture),
    Archive(Archive),
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

fn get_frame_for_turn(game_id: &str, turn: i32) -> Result<Value> {
    let body: Value = ureq::get(
        format!(
            "https://engine.battlesnake.com/games/{}/frames?offset={}&limit=1",
            game_id, turn
        )
        .as_str(),
    )
    .call()?
    .into_json()?;

    Ok(body["Frames"][0].clone())
}

#[derive(Serialize, Deserialize)]
struct FrameResponse {
    #[serde(rename = "Frames")]
    frames: Option<Vec<Value>>,
}

fn get_batch_of_frames_for_games(
    game_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Option<Vec<Value>>> {
    Ok(ureq::get(&format!(
        "https://engine.battlesnake.com/games/{game_id}/frames?offset={offset}&limit={limit}",
    ))
    .call()?
    .into_json::<FrameResponse>()?
    .frames)
}

fn get_frames_for_game(game_id: &str, end_turn: usize) -> Result<Vec<Value>> {
    const LIMIT: usize = 100;
    let mut offset = 0;

    let mut all_frames: Vec<Value> = Vec::with_capacity(end_turn);

    while let Some(frames) = get_batch_of_frames_for_games(game_id, offset, LIMIT)? && !frames.is_empty()
    {
        all_frames.extend(frames.iter().cloned());
        offset += LIMIT;
    }

    Ok(all_frames)
}

fn print_moves<
    GameType: SnakeIDGettableGame<SnakeIDType = SnakeId> + Debug + Clone,
    ScoreType: Copy + Ord + Debug,
>(
    result: &MinMaxReturn<GameType, ScoreType>,
    current_turn: i32,
    m: Move,
) {
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

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    match args.command {
        Commands::Solve(s) => solve(s)?,
        Commands::Fixture(f) => fixture(f)?,
        Commands::Archive(a) => archive(a)?,
    }

    Ok(())
}

fn archive(args: Archive) -> Result<()> {
    let game_id = args.game_id;

    let body: Value = ureq::get(format!("https://engine.battlesnake.com/games/{game_id}").as_str())
        .call()?
        .into_json()?;
    let last_turn = body["LastFrame"]["Turn"].as_i64().unwrap() as usize;

    let frames = get_frames_for_game(&game_id, last_turn)?;

    let games: Result<Vec<Game>, _> = frames
        .iter()
        .map(|f| frame_to_game(f, &body["Game"], &args.you_name))
        .collect();
    let mut games = games?;

    games.sort_by_key(|g| g.turn);

    let document: Result<String, _> = games
        .into_iter()
        .map(|g| serde_json::to_string(&g))
        .collect();

    let mut file = File::create(format!("./archive/{game_id}.jsonl"))?;
    file.write_all(document?.as_bytes())?;

    Ok(())
}
fn fixture(args: Fixture) -> Result<()> {
    let game_id = args.game_id;
    let turn = args.turn;

    let body: Value = ureq::get(format!("https://engine.battlesnake.com/games/{game_id}").as_str())
        .call()?
        .into_json()?;
    let frame = get_frame_for_turn(&game_id, args.turn)?;
    let wire_game = frame_to_game(&frame, &body["Game"], &args.you_name).unwrap();

    let file = File::create(format!("./fixtures/{game_id}_{turn}.json"))?;
    serde_json::to_writer_pretty(file, &wire_game)?;

    dbg!(wire_game);

    Ok(())
}

fn solve(args: Solve) -> Result<()> {
    let body: Value =
        ureq::get(format!("https://engine.battlesnake.com/games/{}", args.game_id).as_str())
            .call()?
            .into_json()?;

    let last_frame = &body["LastFrame"];
    let last_turn = last_frame["Turn"].as_i64().expect("Missing Turn") as i32;
    let mut current_turn = args.search_starting_turn.unwrap_or(last_turn - 1);

    loop {
        let current_frame = get_frame_for_turn(&args.game_id, current_turn)?;
        let wire_game = frame_to_game(&current_frame, &body["Game"], &args.you_name);

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
        let current_frame = get_frame_for_turn(&args.game_id, current_turn)?;
        let wire_game = frame_to_game(&current_frame, &body["Game"], &args.you_name).unwrap();

        if !wire_game.is_wrapped() {
            unimplemented!("Only implementing for wrapped games, RIGHT NOW");
        }

        let snake_ids = build_snake_id_map(&wire_game);
        let game_info = wire_game.game.clone();
        let game: WrappedCellBoard<u16, ArcadeMaze, { 19 * 21 }, 8> =
            wire_game.as_wrapped_cell_board(&snake_ids).unwrap();

        let you_id = game.you_id();

        let explorer_snake =
            MinimaxSnake::from_fn(game, game_info, current_turn, &|_| {}, "explorer");

        let max_turns = (last_living_turn + 1 - current_turn + args.turns_after_lose) as usize;
        let result = explorer_snake.deepend_minimax_to_turn(max_turns);

        let score = *result.score();

        if matches!(score, WrappedScore::Lose(_) | WrappedScore::Tie(_)) {
            println!("At turn {}, there were no safe options", current_turn);
        } else if matches!(score, WrappedScore::Win(_)) {
            println!("At turn {}, you could have won!", current_turn);
            if let MinMaxReturn::Node { options, .. } = &result {
                let winning_moves = options
                    .iter()
                    .filter(|(_, r)| matches!(r.score(), WrappedScore::Win(_)))
                    .map(|(m, _)| *m)
                    .collect_vec();

                println!(
                    "At turn {}, the winning moves were {:?}",
                    current_turn, winning_moves
                );
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

            println!(
                "At turn {}, the safe options were {:?}",
                current_turn, safe_moves
            );
            println!("Turn {} is the decision point", current_turn);

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
