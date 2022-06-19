# battlesnake-rs

This project holds my Snakes which play on play.battlesnake.com!

## Snakes

### [Amphibious Arthur](https://play.battlesnake.com/u/coreyja/amphibious-arthur/)

Arthur was my first snake! He was originally built to be more sole play based, but adopted some
multi-player smarts eventually.

He is a hand-rolled tree-search based snake. He looks at his possible moves and scores them. The
scoring function works by recursively looking a set number of moves ahead and summing all the
scores. From these possible moves he picks the one with the highest score.

### [Bombastic Bob](https://play.battlesnake.com/u/coreyja/bombastic-bob/)

Bob just goes in a random 'reasonable' direction each time. He won't dive into an existing snake or
off the board, but is happy to run into a head to head.

### [Constant Carter](https://play.battlesnake.com/u/coreyja/constant-carter/)

Carter just goes right. Thats all. Mostly created to test latency between the Battlesnake Server
and my snake server.

### [Devious Devin](https://play.battlesnake.com/u/coreyja/devious-devin/)

AKA: Business Snail

Devin is my first minimax snake! Has a pretty highly tuned minimax implementation with an scoring
algorithm that is mostly based on the distance to either food or the closest opponents head.

My current best 'competitive' snake at the moment! Most recently he was invited and competed in the
[Elite Division Winter Classic Invitational 2021](https://play.battlesnake.com/competitions/fall-league-2021/fall-league-2021-elite/brackets/)

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

### [Famished Frank](https://play.battlesnake.com/u/coreyja/famished-frank/)

Frank is famished… He needs food! Once he's full he goes for the corners

Successfully completed the [Occupy 4 Corners Challenge](https://play.battlesnake.com/g/ee518016-997d-4fdf-9354-a73105876174/)

### [Gigantic George](https://play.battlesnake.com/u/coreyja/gigantic-george/)

George wants to get as long as possible

Successfully completed the [Maximum Snake Challenge](https://play.battlesnake.com/g/136ef25f-27b3-4adc-86a8-d57eb3b11877/)

### [Hovering Hobbs](https://play.battlesnake.com/u/coreyja/hovering-hobbs/)

Hobbs is an area control snake. They take the minimax implementation from Devin, and combines it
with a Flood Fill inspired algorithm to try and control more of the board than their opponents.

Hobbs is brand new, and excited to compete in the arenas
