mod commands;
mod unofficial_api;

use color_eyre::eyre::Result;
use commands::Commands;

use std::fmt::Debug;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    args.command.run()?;

    Ok(())
}
