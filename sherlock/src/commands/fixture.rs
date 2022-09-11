use std::fs::File;

use color_eyre::eyre::Result;
use serde_json::Value;

use crate::unofficial_api::{frame_to_game, get_frame_for_turn};

#[derive(clap::Args, Debug)]
pub struct Fixture {
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

impl Fixture {
    pub fn run(self) -> Result<()> {
        let game_id = self.game_id;
        let turn = self.turn;

        let body: Value =
            ureq::get(format!("https://engine.battlesnake.com/games/{game_id}").as_str())
                .call()?
                .into_json()?;
        let frame = get_frame_for_turn(&game_id, self.turn)?;
        let wire_game = frame_to_game(&frame, &body["Game"], &self.you_name).unwrap();

        let file = File::create(format!("./fixtures/{game_id}_{turn}.json"))?;
        serde_json::to_writer_pretty(file, &wire_game)?;

        dbg!(wire_game);

        Ok(())
    }
}
