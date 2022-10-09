use color_eyre::eyre::Result;
use scraper::{Html, Selector};

use crate::commands::archive_snake::ArchiveSnake;

use super::archive::ArchiveShared;

#[derive(clap::Args, Debug)]
pub(crate) struct ArchiveUser {
    /// The URL for the snake to archive
    #[clap(short, long, value_parser)]
    user_url: String,

    #[clap(flatten)]
    shared: ArchiveShared,
}

impl ArchiveUser {
    pub(crate) fn run(self) -> Result<()> {
        let res = ureq::get(&self.user_url).call()?;
        let html_string = res.into_string()?;
        let document = Html::parse_document(&html_string);

        let selector =
            Selector::parse("#tab-battlesnakes .list-group-item p:first-of-type a").unwrap();

        for element in document.select(&selector) {
            let href = element
                .value()
                .attr("href")
                .expect("No URL found")
                .to_string();
            assert!(href.starts_with("/u/"));

            let snake_url = format!("https://play.battlesnake.com{href}");

            ArchiveSnake::new(snake_url, self.shared.clone()).run()?
        }

        Ok(())
    }
}
