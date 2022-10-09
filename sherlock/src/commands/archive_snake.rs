use color_eyre::eyre::Result;
use colored::Colorize;
use scraper::{Html, Selector};

use crate::commands::archive::Archive;

use super::archive::ArchiveShared;

#[derive(clap::Args, Debug)]
pub(crate) struct ArchiveSnake {
    /// The URL for the snake to archive
    #[clap(short, long, value_parser)]
    snake_url: String,

    #[clap(flatten)]
    shared: ArchiveShared,
}

impl ArchiveSnake {
    pub(crate) fn run(self) -> Result<()> {
        let res = ureq::get(&self.snake_url).call()?;
        let html_string = res.into_string()?;
        let document = Html::parse_document(&html_string);

        let snake_name: String = document
            .select(&Selector::parse(".page-header h1").unwrap())
            .next()
            .unwrap()
            .text()
            .collect();

        println!(
            "{}",
            format!("⏳🐍 Archive in progress for {snake_name}").yellow()
        );

        for element in document.select(&Selector::parse(".list-group-item a").unwrap()) {
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

            Archive::new(game_id, self.shared.clone()).run()?
        }

        Ok(())
    }

    pub fn new(snake_url: String, shared: ArchiveShared) -> Self {
        Self { snake_url, shared }
    }
}
