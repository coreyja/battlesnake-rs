# battlesnake-rs

This project holds my Snakes which play on play.battlesnake.com!

## Snakes

### [Eremetic Eric](https://play.battlesnake.com/u/coreyja/eremetic-eric/)

#### Strategy

Eric is a single player snake. His goal is to survive as many turns as possible.

To do this he does a lot of tail chasing. The high level strategy goes something like the following:

- Use A* to find the shortest path to our tail
- Use this path to pretend the snake is a complete 'circle', IE: Change the board such that our snake contains all pieces from the A* search we just did
- If our health is greater than the 'completed circle' snake length, keep circling until we are low on health
- When we are low on health, look for the "best" food item, where best is defined as follows:
  - We can get to this food with the lowest possible health, such that we prioritize using as much health as possible
- Once we have gotten determined which food is best, we determine which body piece is where we should 'exit' our circle to grab the food. This is the body piece where A* finds the shortest distance to the chosen food.
- If we are at this chosen body piece, follow the A* prime to the food
- If we are NOT at this chosen body piece, keep looping until we are

Eric is open for games on play.battlesnake.com, so feel free to start a Solo game with him and watch what he does!
