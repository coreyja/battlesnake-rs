pub mod archive;
pub mod archive_snake;
pub mod archive_user;
pub mod fixture;
pub mod replay;
pub mod solve;

use archive::Archive;
use archive_snake::ArchiveSnake;
use archive_user::ArchiveUser;
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
    ArchiveSnake(ArchiveSnake),
    ArchiveUser(ArchiveUser),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Command::Solve(s) => s.run()?,
            Command::Fixture(f) => f.run()?,
            Command::Archive(a) => a.run()?,
            Command::Replay(r) => r.run()?,
            Command::ArchiveSnake(a) => a.run()?,
            Command::ArchiveUser(a) => a.run()?,
        }

        Ok(())
    }
}
