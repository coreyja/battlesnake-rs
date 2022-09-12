use std::path::PathBuf;

use color_eyre::eyre::Result;
use scraper::{Html, Selector};

use crate::commands::archive::archive;

#[derive(clap::Args, Debug)]
pub(crate) struct ArchiveAll {
    /// The URL for the snake to archive
    #[clap(short, long, value_parser)]
    snake_url: String,

    /// Directory to archive games to
    #[clap(short, long, value_parser)]
    archive_dir: PathBuf,
}

impl ArchiveAll {
    pub(crate) fn run(self) -> Result<()> {
        let res = ureq::get(&self.snake_url).call()?;
        let html_string = res.into_string()?;
        let document = Html::parse_document(&html_string);

        let selector = Selector::parse(".list-group-item a").unwrap();

        for element in document.select(&selector) {
            let url = element
                .value()
                .attr("href")
                .expect("No URL found")
                .to_string();
            assert!(url.starts_with("/g/"));
            let game_id = {
                let game_id = url.strip_prefix("/g/").unwrap();
                let game_id = game_id.strip_suffix('/').unwrap();
                game_id.to_string()
            };

            archive(game_id, self.archive_dir.clone())?
        }

        Ok(())
    }
}
