use super::*;

pub struct DeviousDevin {}

#[derive(Serialize)]
pub struct MoveOption {
    moves: Vec<SnakeMove>,
    score: ScoreEndState,
    dir: Direction,
}

#[derive(Serialize)]
pub struct EvaluateOutput {
    options: Vec<MoveOption>,
}

fn moves_to_my_direction(moves: &Vec<SnakeMove>, game_state: &GameState) -> Direction {
    moves
        .iter()
        .find(|m| m.snake_id == game_state.you.id)
        .map(|m| m.dir)
        .unwrap_or(
            game_state.you.body[0]
                .possible_moves(&game_state.board)
                .get(0)
                .map(|x| x.0)
                .unwrap_or(Direction::UP),
        )
}

impl DeviousDevin {
    pub fn explain_move(
        &self,
        game_state: GameState,
    ) -> Result<EvaluateOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut game_state = game_state;
        let mut sorted_snakes = game_state.board.snakes.clone();
        sorted_snakes.sort_by_key(|snake| if snake.id == game_state.you.id { 1 } else { -1 });

        let options = minimax_options(
            &mut game_state,
            &sorted_snakes,
            0,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            vec![],
        );

        let mut options: Vec<MoveOption> = options
            .into_iter()
            .map(|(score, moves)| {
                let dir = moves_to_my_direction(&moves, &game_state);

                MoveOption { score, moves, dir }
            })
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
        let mut sorted_snakes = game_state.board.snakes.clone();
        sorted_snakes.sort_by_key(|snake| if snake.id == game_state.you.id { 1 } else { -1 });

        let (_, moves) = minimax(
            &mut game_state,
            &sorted_snakes,
            0,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            vec![],
        );

        Ok(MoveOutput {
            r#move: moves_to_my_direction(&moves, &game_state).value(),
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

#[derive(Serialize, PartialEq, PartialOrd, Ord, Eq, Debug, Copy, Clone)]
enum ScoreEndState {
    /// depth: i64
    HitSelfLose(i64),
    /// depth: i64
    RanIntoOtherLose(i64),
    /// depth: i64
    OutOfHealthLose(i64),
    /// depth: i64
    HeadToHeadLose(i64),
    /// difference_in_snake_length: i64, negaitve_distance_to_nearest_food: Option<i64>, health: u8
    ShorterThanOpponent(i64, Option<i64>, i16),
    /// negative_distance_to_opponent: Option<i64>, difference_in_snake_length: i64, health: u8
    LongerThanOpponent(Option<i64>, i64, i16),
    /// negative_depth: i64
    HitSelfWin(i64),
    /// negative_depth: i64
    RanIntoOtherWin(i64),
    /// negative_depth: i64
    HeadToHeadWin(i64),
}

const BEST_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::HeadToHeadWin(i64::MAX);
const WORT_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::HitSelfLose(i64::MIN);

fn score(node: &GameState, depth: i64) -> Option<ScoreEndState> {
    let num_snakes: i64 = node.board.snakes.len().try_into().unwrap();
    if depth % num_snakes != 0 {
        return None;
    }

    let max_depth: i64 = match num_snakes {
        2 => 10,
        3 => 9,
        4 => 8,
        5 => 10,
        _ => 10,
    };

    let me: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == node.you.id)
        .unwrap();
    let opponents: Vec<&Battlesnake> = node
        .board
        .snakes
        .iter()
        .filter(|s| s.id != node.you.id)
        .collect();

    let oppenent_heads: Vec<Coordinate> = opponents.iter().map(|s| s.body[0].clone()).collect();

    let my_length: i64 = me.body.len().try_into().unwrap();

    if me.body[1..].contains(&me.body[0]) && depth != 0 {
        return Some(ScoreEndState::HitSelfLose(depth));
    }

    if me.health <= 0 {
        return Some(ScoreEndState::OutOfHealthLose(depth));
    }

    if let Some(opponent_end_state) = opponents
        .iter()
        .filter_map(|not_me| {
            if not_me.body[1..].contains(&me.body[0]) {
                return Some(ScoreEndState::RanIntoOtherLose(depth));
            }

            if not_me.body[1..].contains(&not_me.body[0]) && depth != 0 {
                return Some(ScoreEndState::HitSelfWin(-depth));
            }

            if me.body[1..].contains(&not_me.body[0]) {
                return Some(ScoreEndState::RanIntoOtherWin(-depth));
            }

            if me.body[0] == not_me.body[0] {
                if my_length > not_me.body.len().try_into().unwrap() {
                    return Some(ScoreEndState::HeadToHeadWin(-depth));
                } else {
                    return Some(ScoreEndState::HeadToHeadLose(depth));
                }
            }

            None
        })
        .min()
    {
        return Some(opponent_end_state);
    };

    if depth >= max_depth {
        let max_opponent_length: i64 = opponents
            .iter()
            .map(|s| s.body.len().try_into().unwrap())
            .max()
            .unwrap();
        let length_difference = my_length - max_opponent_length;

        if max_opponent_length >= my_length || me.health < 20 {
            let negative_closest_food_distance =
                a_prime::shortest_distance(&node.board, &me.body[0], &node.board.food).map(|x| -x);

            return Some(ScoreEndState::ShorterThanOpponent(
                length_difference,
                negative_closest_food_distance,
                me.health.max(50),
            ));
        }

        let negative_distance_to_opponent =
            a_prime::shortest_distance(&node.board, &me.body[0], &oppenent_heads).map(|dist| -dist);

        return Some(ScoreEndState::LongerThanOpponent(
            negative_distance_to_opponent,
            length_difference.max(4),
            me.health.max(50),
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
    snakes: &[Battlesnake],
    depth: usize,
    alpha: ScoreEndState,
    beta: ScoreEndState,
    current_moves: Vec<SnakeMove>,
) -> (ScoreEndState, Vec<SnakeMove>) {
    let options = minimax_options(node, snakes, depth, alpha, beta, current_moves);

    let snake = &snakes[depth % snakes.len()];
    let is_maximizing = snake.id == node.you.id;

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
    snakes: &[Battlesnake],
    depth: usize,
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

    let snake = &snakes[depth % snakes.len()];
    let is_maximizing = snake.id == node.you.id;

    for (dir, coor) in children(node, &snake.id).into_iter() {
        let last_move = node.move_to(&coor, &snake.id);
        let new_current_moves = {
            let mut x = current_moves.clone();
            x.push(SnakeMove {
                dir,
                snake_name: snake.name.clone(),
                snake_id: snake.id.clone(),
                move_to: coor.clone(),
            });
            x
        };
        let next_move_return = minimax(node, snakes, depth + 1, alpha, beta, new_current_moves);
        let value = next_move_return.0;
        node.reverse_move(last_move);
        options.push(next_move_return);

        if is_maximizing {
            alpha = std::cmp::max(alpha, value);
            if beta <= alpha {
                break;
            }
        } else {
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
