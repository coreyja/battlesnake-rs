pub mod archive;
pub mod fixture;
pub mod replay;
pub mod solve;

use archive::Archive;
use fixture::Fixture;
use replay::Replay;
use solve::Solve;

use clap::Subcommand;
use color_eyre::eyre::Result;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Solve(Solve),
    Fixture(Fixture),
    Archive(Archive),
    Replay(Replay),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Command::Solve(s) => s.run()?,
            Command::Fixture(f) => f.run()?,
            Command::Archive(a) => a.run()?,
            Command::Replay(r) => r.run()?,
        }

        Ok(())
    }
}
