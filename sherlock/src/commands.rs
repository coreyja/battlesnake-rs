pub mod archive;
pub mod fixture;
pub mod solve;

use archive::Archive;
use fixture::Fixture;
use solve::Solve;

use clap::Subcommand;
use color_eyre::eyre::Result;

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Solve(Solve),
    Fixture(Fixture),
    Archive(Archive),
}

impl Commands {
    pub fn run(self) -> Result<()> {
        match self {
            Commands::Solve(s) => s.run()?,
            Commands::Fixture(f) => f.run()?,
            Commands::Archive(a) => a.run()?,
        }

        Ok(())
    }
}
