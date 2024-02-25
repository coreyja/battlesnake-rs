# Sherlock

Sherlock is a standalone binary that provides various tools that may be useful for a Battlesnake Developer.

Some of them may be documented here

## Replaying Games Howto

### Install Sherlock and move it to a place on your path

```bash
# PreReq Install Rust
# Rustup will help you there!
# https://rustup.rs/

# Clone the repo
git clone https://github.com/coreyja/battlesnake-rs.git
cd battlesnake-rs/sherlock

cargo build --release

cp ../target/release/sherlock ~/bin/
```

### Archive a game

```bash
sherlock archive --game-id 'GAME_ID_HERE'
```

This will save an archive of the game to `./archive`

### Replay an archive game

```bash
sherlock replay archive
```

This will start the replay server on `localhost:8085`


Now you just need to board a Board instance at your local replay server.
We can even use the live board for this!

We can navigate to something like the following to view a game we have saved in our archive!

```
https://board.battlesnake.com/?engine=http://localhost:8085&game=GAME_ID
```
