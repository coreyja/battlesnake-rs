use battlesnake_minimax::types::types::SnakeIDGettableGame;
use battlesnake_rs::{HeadGettableGame, HealthGettableGame, Move, Vector};

use crate::*;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct AppState {
    pub game_states: HashMap<String, GameState>,
}

#[derive(Debug, Clone)]
pub(crate) struct GameState {
    pub last_move: Option<LastMoveState>,
    pub id_map: HashMap<String, SnakeId>,
}

#[derive(Debug, Clone)]
pub(crate) struct LastMoveState {
    pub last_return: MinMaxReturn<WrappedCellBoard4Snakes11x11, Score>,
    pub last_board: WrappedCellBoard4Snakes11x11,
    pub turn: i32,
}

impl GameState {
    pub fn new(id_map: HashMap<String, SnakeId>) -> Self {
        Self {
            last_move: None,
            id_map,
        }
    }
}
pub(crate) async fn route_hobbs_info() -> impl IntoResponse {
    Json(Factory {}.about())
}
pub(crate) async fn route_hobbs_start(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let id_map = build_snake_id_map(&game);
    let mut state = state.lock().unwrap();
    state
        .game_states
        .insert(game.game.id, GameState::new(id_map));
    StatusCode::NO_CONTENT
}
pub(crate) async fn route_hobbs_end() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub(crate) async fn route_hobbs_move(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let game_info = game.game.clone();
    let game_id = game_info.id.to_string();
    let turn = game.turn;

    let name = "hovering-hobbs";

    let options: SnakeOptions = SnakeOptions {
        network_latency_padding: Duration::from_millis(50),
        move_ordering: MoveOrdering::BestFirst,
    };

    let game_state = {
        let state_guard = state.lock().unwrap();

        state_guard
            .game_states
            .get(&game_id)
            .expect("If we hit the start endpoint we should have a game state already")
            .clone()
    };
    let last_move = &game_state.last_move;

    let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &game_state.id_map)
        .expect("TODO: We need to work on our error handling");

    let you_id = game.you_id();

    let initial_return = if let Some(last_move) = last_move && last_move.turn == turn - 1 {
        let last_board = &last_move.last_board;
        let previously_alive_snakes = game_state
            .id_map
            .values()
            .filter(|sid| last_board.is_alive(sid));

        let previous_heads: HashMap<&SnakeId, _> = previously_alive_snakes
            .map(|sid| (sid, last_board.get_head_as_position(sid)))
            .collect();

        let current_snake_ids = game.get_snake_ids();
        let currently_alive_snakes = current_snake_ids.iter().filter(|sid| game.is_alive(sid));
        let current_heads = currently_alive_snakes.map(|sid| (sid, game.get_head_as_position(sid)));

        let mut snake_moves = HashMap::new();

        for (sid, head) in current_heads {
            let previous_head = previous_heads
                .get(sid)
                .expect("If you are alive now you better have had a head last turn");
            let previous_head_vector = previous_head.to_vector();
            let current_head_vector = head.to_vector();

            let x_diff = current_head_vector.x - previous_head_vector.x;
            let x_diff = match x_diff {
                10 => -1,
                -10 => 1,
                x => x,
            };
            let y_diff = current_head_vector.y - previous_head_vector.y;
            let y_diff = match y_diff {
                10 => -1,
                -10 => 1,
                x => x,
            };

            let move_vector = Vector {
                x: x_diff,
                y: y_diff,
            };

            let m = Move::from_vector(move_vector);

            snake_moves.insert(sid, m);
        }

        let mut current_return = last_move.last_return.clone();

        while
            let Some(moving_snake_id) = current_return.moving_snake_id() &&
            let Some(m) = snake_moves.remove(moving_snake_id) &&
            let Some(next_return) = current_return.option_for_move(m)
            {
            current_return = next_return.clone();
        }

        while
            let MinMaxReturn::Node { ref options, moving_snake_id, .. } = current_return &&
            moving_snake_id == *you_id {
                let new_return = options[0].1.clone();
                current_return = new_return;
        }

        Some(current_return)
    } else {

        None
    };

    let my_id = game.you_id();
    let snake = ParanoidMinimaxSnake::new(game, game_info, turn, &standard_score, name, options);

    let (_depth, scored) =
        spawn_blocking_with_tracing(move || snake.choose_move_inner(initial_return))
            .await
            .unwrap();

    let scored_options = scored.first_options_for_snake(my_id).unwrap();
    let output = scored_options.first().unwrap().0;

    {
        let mut state = state.lock().unwrap();

        let mut game_state = state
            .game_states
            .get_mut(&game_id)
            .expect("If we hit the start endpoint we should have a game state already");

        let last_move = LastMoveState {
            last_return: scored,
            last_board: game,
            turn,
        };
        game_state.last_move = Some(last_move);
    }

    let output: MoveOutput = MoveOutput {
        r#move: format!("{output}"),
        shout: None,
    };

    Json(output)
}
