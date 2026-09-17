use std::collections::{HashSet, VecDeque};

use battlesnake_game_types::wire_representation::{BattleSnake, Game, Position};
use serde::Serialize;

pub(super) const DIRECTIONS: [(&str, i32, i32); 4] = [
    ("up", 0, 1),
    ("down", 0, -1),
    ("left", -1, 0),
    ("right", 1, 0),
];

#[derive(Debug, Serialize)]
pub(super) struct Candidate {
    pub direction: &'static str,
    pub destination: Position,
    pub health_after_move: i32,
    pub eats_food: bool,
    pub losing_head_to_head_risk: bool,
    pub reachable_cells: usize,
    pub cramped_region: bool,
    pub steps_to_food: Option<usize>,
}

fn constrictor(game: &Game) -> bool {
    game.game.ruleset.name == "constrictor"
}

fn destination(game: &Game, head: Position, dx: i32, dy: i32) -> Option<Position> {
    let mut next = Position::new(head.x + dx, head.y + dy);
    if game.is_wrapped() && game.board.width > 0 && game.board.height > 0 {
        next.x = next.x.rem_euclid(game.board.width as i32);
        next.y = next.y.rem_euclid(game.board.height as i32);
    }
    (!game.off_board(next)).then_some(next)
}

fn blocked_cells(game: &Game) -> HashSet<Position> {
    game.board
        .snakes
        .iter()
        .flat_map(|snake| {
            // Standard movement removes the old tail before feeding. Stacked tails
            // remain occupied because the preceding segment is still in this set.
            let retained = snake
                .body
                .len()
                .saturating_sub(usize::from(!constrictor(game)));
            snake.body.iter().take(retained).copied()
        })
        .collect()
}

fn health_after_move(game: &Game, snake: &BattleSnake, next: Position) -> i32 {
    if constrictor(game) || game.board.food.contains(&next) {
        return 100;
    }
    let damage = game
        .game
        .ruleset
        .settings
        .as_ref()
        .map_or(0, |s| s.hazard_damage_per_turn);
    let mut health = snake.health - 1;
    // Stacked hazards each apply damage. Food suppresses hazard damage entirely.
    for _ in game.board.hazards.iter().filter(|&&p| p == next) {
        health = health.saturating_sub(damage).clamp(0, 100);
        if health == 0 {
            break;
        }
    }
    health
}

fn open_moves<'a>(
    game: &'a Game,
    snake: &'a BattleSnake,
    blocked: &'a HashSet<Position>,
) -> impl Iterator<Item = (&'static str, Position)> + 'a {
    DIRECTIONS.into_iter().filter_map(move |(name, dx, dy)| {
        let next = destination(game, snake.head, dx, dy)?;
        // A length-two snake's tail is also its neck: reversing is still forbidden.
        (snake.body.get(1) != Some(&next)
            && !blocked.contains(&next)
            && health_after_move(game, snake, next) > 0)
            .then_some((name, next))
    })
}

fn region(game: &Game, start: Position, blocked: &HashSet<Position>) -> (usize, Option<usize>) {
    let mut visited = HashSet::from([start]);
    let mut queue = VecDeque::from([(start, 0)]);
    let mut food_distance = None;
    while let Some((pos, distance)) = queue.pop_front() {
        if food_distance.is_none() && game.board.food.contains(&pos) {
            food_distance = Some(distance);
        }
        for (_, dx, dy) in DIRECTIONS {
            if let Some(next) = destination(game, pos, dx, dy)
                && !blocked.contains(&next)
                && visited.insert(next)
            {
                queue.push_back((next, distance + 1));
            }
        }
    }
    (visited.len(), food_distance)
}

pub(super) fn candidates(game: &Game) -> Vec<Candidate> {
    let blocked = blocked_cells(game);
    let mut moves: Vec<_> = open_moves(game, &game.you, &blocked)
        .map(|(direction, next)| {
            let (reachable_cells, steps_to_food) = region(game, next, &blocked);
            let losing_head_to_head_risk = game.board.snakes.iter().any(|opponent| {
                opponent.id != game.you.id
                    && opponent.body.len() >= game.you.body.len()
                    && open_moves(game, opponent, &blocked).any(|(_, pos)| pos == next)
            });
            let eats_food = game.board.food.contains(&next);
            let length_after_move =
                game.you.body.len() + usize::from(eats_food || constrictor(game));
            Candidate {
                direction,
                destination: next,
                health_after_move: health_after_move(game, &game.you, next),
                eats_food,
                losing_head_to_head_risk,
                reachable_cells,
                cramped_region: reachable_cells < length_after_move,
                steps_to_food,
            }
        })
        .collect();
    if moves.iter().any(|m| !m.losing_head_to_head_risk) {
        moves.retain(|m| !m.losing_head_to_head_risk);
    }
    moves
}

pub(super) fn fallback(game: &Game, moves: &[Candidate]) -> &'static str {
    moves
        .iter()
        .max_by_key(|m| {
            let food = -(m.steps_to_food.unwrap_or(10_000) as i64);
            (
                !m.losing_head_to_head_risk,
                !m.cramped_region,
                if game.you.health < 35 { food } else { 0 },
                m.reachable_cells,
                m.health_after_move,
                food,
            )
        })
        .map(|m| m.direction)
        .unwrap_or_else(|| {
            // No move survives our immediate checks. Still return a valid direction,
            // preferring an in-bounds move that is not a reversal.
            DIRECTIONS
                .into_iter()
                .find_map(|(name, dx, dy)| {
                    let next = destination(game, game.you.head, dx, dy)?;
                    (game.you.body.get(1) != Some(&next)).then_some(name)
                })
                .unwrap_or("up")
        })
}

#[cfg(test)]
mod tests;
