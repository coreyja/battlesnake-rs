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
    #[allow(dead_code)]
    pub last_board: WrappedCellBoard4Snakes11x11,
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
        let state = state.lock().unwrap();

        let game_state = state
            .game_states
            .get(&game_id)
            .expect("If we hit the start endpoint we should have a game state already");
        let last_move = &game_state.last_move;

        if let Some(r) = last_move {
            dbg!("We found a last return");
            dbg!(r.last_return.score());
        } else {
            dbg!("What this the first turn of the game? No last return found");
        }

        game_state.clone()
    };

    let game: WrappedCellBoard4Snakes11x11 =
        WrappedCellBoard4Snakes11x11::convert_from_game(game, &game_state.id_map)
            .expect("TODO: We need to work on our error handling");
    let my_id = game.you_id();
    let snake = ParanoidMinimaxSnake::new(game, game_info, turn, &standard_score, name, options);

    let (_depth, scored) = spawn_blocking_with_tracing(move || snake.choose_move_inner())
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
        };
        game_state.last_move = Some(last_move);
    }

    let output: MoveOutput = MoveOutput {
        r#move: format!("{output}"),
        shout: None,
    };

    Json(output)
}
