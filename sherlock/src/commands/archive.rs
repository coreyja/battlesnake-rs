use std::{fs::File, io::Write};

use color_eyre::eyre::Result;
use serde_json::Value;
use types::wire_representation::Game;

use crate::unofficial_api::{frame_to_game, get_frames_for_game};

#[derive(clap::Args, Debug)]
pub(crate) struct Archive {
    /// Game ID to debug
    #[clap(short, long, value_parser)]
    game_id: String,

    /// The name of the snake to use as "you"
    #[clap(short, long, value_parser)]
    you_name: String,
}

impl Archive {
    pub(crate) fn run(self) -> Result<()> {
        let game_id = self.game_id;

        let body: Value =
            ureq::get(format!("https://engine.battlesnake.com/games/{game_id}").as_str())
                .call()?
                .into_json()?;
        let last_turn = body["LastFrame"]["Turn"].as_i64().unwrap() as usize;

        let frames = get_frames_for_game(&game_id, last_turn)?;

        let games: Result<Vec<Game>, _> = frames
            .iter()
            .map(|f| frame_to_game(f, &body["Game"], &self.you_name))
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
}
