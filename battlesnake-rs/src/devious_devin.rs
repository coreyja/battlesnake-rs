use super::*;

pub struct DeviousDevin {}

#[derive(Serialize)]
pub struct MoveOption {
    moves: Vec<SnakeMove>,
    score: ScoreEndState,
}

#[derive(Serialize)]
pub struct EvaluateOutput {
    options: Vec<MoveOption>,
}

impl DeviousDevin {
    pub fn explain_move(
        &self,
        game_state: GameState,
    ) -> Result<EvaluateOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut game_state = game_state;
        let options = minimax_options(
            &mut game_state,
            0,
            true,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            vec![],
        );

        let mut options: Vec<MoveOption> = options
            .into_iter()
            .map(|(score, moves)| MoveOption { score, moves })
            .collect();

        options.sort_by_key(|option| option.score);
        options.reverse();

        Ok(EvaluateOutput { options })
    }
}

impl BattlesnakeAI for DeviousDevin {
    fn make_move(
        &self,
        game_state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut game_state = game_state;
        let (_score, moves) = minimax(
            &mut game_state,
            0,
            true,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            vec![],
        );

        Ok(MoveOutput {
            r#move: moves.get(0).unwrap().dir.value(),
            shout: None,
        })
    }

    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#99cc00".to_owned()),
            head: Some("snail".to_owned()),
            tail: Some("rbc-necktie".to_owned()),
            version: None,
        }
    }
}

const MAX_DEPTH: i64 = 10;

#[derive(Serialize, PartialEq, PartialOrd, Ord, Eq, Debug, Copy, Clone)]
enum ScoreEndState {
    /// depth: i64
    HitSelfLose(i64),
    /// depth: i64
    RanIntoOtherLose(i64),
    /// depth: i64
    HeadToHeadLose(i64),
    /// difference_in_snake_length: i64, negaitve_distance_to_nearest_food: Option<i64>, health: u8
    ShorterThanOpponent(i64, Option<i64>, u8),
    /// negative_distance_to_opponent: Option<i64>, difference_in_snake_length: i64, health: u8
    LongerThanOpponent(Option<i64>, i64, u8),
    /// depth: i64
    HitSelfWin(i64),
    /// depth: i64
    RanIntoOtherWin(i64),
    /// depth: i64
    HeadToHeadWin(i64),
}

const BEST_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::HeadToHeadWin(i64::MAX);
const WORT_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::HitSelfLose(i64::MIN);

fn score(node: &GameState, depth: i64) -> Option<ScoreEndState> {
    let num_snakes: i64 = node.board.snakes.len().try_into().unwrap();
    if depth % num_snakes != 0 {
        return None;
    }

    let me: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == node.you.id)
        .unwrap();
    let not_me = node
        .board
        .snakes
        .iter()
        .find(|s| s.id != node.you.id)
        .unwrap();

    let my_length: i64 = me.body.len().try_into().unwrap();
    let not_my_length: i64 = not_me.body.len().try_into().unwrap();

    if me.body[1..].contains(&me.body[0]) && depth != 0 {
        return Some(ScoreEndState::HitSelfLose(depth));
    }

    if not_me.body[1..].contains(&me.body[0]) {
        return Some(ScoreEndState::RanIntoOtherLose(depth));
    }

    if not_me.body[1..].contains(&not_me.body[0]) && depth != 0 {
        return Some(ScoreEndState::HitSelfWin(depth));
    }

    if me.body[1..].contains(&not_me.body[0]) {
        return Some(ScoreEndState::RanIntoOtherWin(depth));
    }

    if me.body[0] == not_me.body[0] {
        if my_length > not_my_length {
            return Some(ScoreEndState::HeadToHeadWin(depth));
        } else {
            return Some(ScoreEndState::HeadToHeadLose(depth));
        }
    }

    if depth == MAX_DEPTH {
        let length_difference = my_length - not_my_length;

        if not_my_length >= my_length {
            let negative_closest_food_distance =
                a_prime::shortest_distance(&node.board, &me.body[0], &node.board.food).map(|x| -x);

            return Some(ScoreEndState::ShorterThanOpponent(
                length_difference,
                negative_closest_food_distance,
                me.health,
            ));
        }

        let negative_distance_to_opponent =
            a_prime::shortest_distance(&node.board, &me.body[0], &vec![not_me.body[0]])
                .map(|dist| -dist);

        return Some(ScoreEndState::LongerThanOpponent(
            negative_distance_to_opponent,
            length_difference,
            me.health,
        ));
    }

    None
}

fn children(node: &GameState, turn_snake_id: &str) -> Vec<(Direction, Coordinate)> {
    let you: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == turn_snake_id)
        .expect("We didn't find that snake");
    you.body[0]
        .possible_moves(&node.board)
        .iter()
        .cloned()
        .collect()
}
use std::convert::TryInto;

fn minimax(
    node: &mut GameState,
    depth: usize,
    is_maximizing: bool,
    alpha: ScoreEndState,
    beta: ScoreEndState,
    current_moves: Vec<SnakeMove>,
) -> (ScoreEndState, Vec<SnakeMove>) {
    let options = minimax_options(node, depth, is_maximizing, alpha, beta, current_moves);

    if is_maximizing {
        options
            .into_iter()
            .max_by_key(|(score, _)| score.clone())
            .unwrap()
    } else {
        options
            .into_iter()
            .min_by_key(|(score, _)| score.clone())
            .unwrap()
    }
}

#[derive(Clone, Debug, Serialize)]
struct SnakeMove {
    snake_name: String,
    snake_id: String,
    dir: Direction,
    move_to: Coordinate,
}

fn minimax_options(
    node: &mut GameState,
    depth: usize,
    is_maximizing: bool,
    alpha: ScoreEndState,
    beta: ScoreEndState,
    current_moves: Vec<SnakeMove>,
) -> Vec<(ScoreEndState, Vec<SnakeMove>)> {
    let mut alpha = alpha;
    let mut beta = beta;

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = score(&node, new_depth) {
        return vec![(s, current_moves)];
    }

    let mut options = vec![];

    if is_maximizing {
        let me = node.you.id.to_owned();
        for (dir, coor) in children(node, &node.you.id).into_iter() {
            let last_move = node.move_to(&coor, &me);
            let new_current_moves = {
                let mut x = current_moves.clone();
                x.push(SnakeMove {
                    dir,
                    snake_name: node.you.name.clone(),
                    snake_id: node.you.id.clone(),
                    move_to: coor.clone(),
                });
                x
            };
            let next_move_return = minimax(node, depth + 1, false, alpha, beta, new_current_moves);
            let value = next_move_return.0;
            node.reverse_move(last_move);

            options.push(next_move_return);
            alpha = std::cmp::max(alpha, value);
            if beta <= alpha {
                break;
            }
        }
    } else {
        let not_me = node
            .board
            .snakes
            .iter()
            .cloned()
            .filter(|s| s.id != node.you.id)
            .next()
            .unwrap();
        for (dir, coor) in children(node, &not_me.id).into_iter() {
            let last_move = node.move_to(&coor, &not_me.id);
            let new_current_moves = {
                let mut x = current_moves.clone();
                x.push(SnakeMove {
                    dir,
                    snake_name: not_me.name.clone(),
                    snake_id: not_me.id.clone(),
                    move_to: coor.clone(),
                });
                x
            };
            let next_move_return = minimax(node, depth + 1, true, alpha, beta, new_current_moves);
            let value = next_move_return.0;
            node.reverse_move(last_move);

            options.push(next_move_return);
            beta = std::cmp::min(beta, value);
            if beta <= alpha {
                break;
            }
        }
    }

    options
}
#[cfg(test)]
mod tests {}
