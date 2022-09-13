#!/usr/bin/env bash

while true
do
  cargo run --bin sherlock -- archive-snake --snake-url https://play.battlesnake.com/u/coreyja/tbd-mcts/
  cargo run --bin sherlock -- archive-snake --snake-url https://play.battlesnake.com/u/coreyja/hovering-hobbs/
  cargo run --bin sherlock -- archive-snake --snake-url https://play.battlesnake.com/u/jonathanarns/shapeshifter/
  cargo run --bin sherlock -- archive-snake --snake-url https://play.battlesnake.com/u/jlafayette/snakebeard/
  sleep 600
done
