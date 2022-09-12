use std::{fs::File, io::Write, path::PathBuf};

use color_eyre::eyre::Result;
use colored::Colorize;
use serde_json::Value;

use crate::{unofficial_api::get_frames_for_game, websockets::get_raw_messages_from_game};

#[derive(clap::Args, Debug)]
pub(crate) struct Archive {
    /// Game ID to debug
    #[clap(short, long, value_parser)]
    game_id: String,

    #[clap(flatten)]
    shared: ArchiveShared,
}

#[derive(clap::Args, Debug, Clone, Default)]
pub(crate) struct ArchiveShared {
    /// Directory to archive games to
    #[clap(short, long, value_parser, default_value = "archive")]
    archive_dir: PathBuf,

    /// Ignores local results and overwrite. Defaults to false
    #[clap(long, action, default_value = "false")]
    force: bool,
}

impl Archive {
    pub fn new(game_id: String, shared: ArchiveShared) -> Self {
        Self { game_id, shared }
    }

    pub(crate) fn run(self) -> Result<()> {
        let game_id = self.game_id;

        let game_dir = self.shared.archive_dir.join(&game_id);
        let game_info_path = game_dir.join("info.json");

        if game_info_path.is_file() && !self.shared.force {
            println!("🎉 Archive already exists for {game_id}");

            return Ok(());
        }

        println!(
            "{}",
            format!("⏳ Archive in progress for {game_id}").yellow()
        );

        let game_details: Value =
            ureq::get(format!("https://engine.battlesnake.com/games/{game_id}").as_str())
                .call()?
                .into_json()?;
        let last_turn = game_details["LastFrame"]["Turn"].as_i64().unwrap() as usize;

        let frames = get_frames_for_game(&game_id, last_turn)?;

        std::fs::create_dir_all(game_dir.as_path())?;

        // Archive the Info 'raw' from the API
        {
            let contents = serde_json::to_string(&game_details)?;
            let mut file = File::create(game_info_path)?;
            file.write_all(contents.as_bytes())?;
        }

        // Archive the Frames 'raw' from the API
        {
            let frame_document: Result<String, _> =
                frames.iter().map(|g| serde_json::to_string(&g)).collect();
            let mut file = File::create(game_dir.join("frames.jsonl"))?;
            file.write_all(frame_document?.as_bytes())?;
        }

        // Archive the 'raw' WebSockets messages
        {
            let websocket_messages = get_raw_messages_from_game(&game_id)?;

            let document = websocket_messages.join("\n");
            let mut file = File::create(game_dir.join("websockets.jsonl"))?;
            file.write_all(document.as_bytes())?;
        }

        println!(
            "{}",
            format!("✔️ Archive created for game {game_id}").green()
        );

        Ok(())
    }
}
