use std::time::Instant;

use battlesnake_game_types::types::Move;
use battlesnake_minimax::types::types::SnakeIDGettableGame;
use battlesnake_rs::{HeadGettableGame, HealthGettableGame, LengthGettableGame, Vector};
use parking_lot::Mutex;

use crate::*;

/// How long a game may go without a request before its state is dropped.
///
/// `/end` is not guaranteed: an engine that errors or restarts mid-game never
/// sends it, and those states used to accumulate until the process was
/// OOM-killed.
const GAME_STATE_IDLE_TTL: Duration = Duration::from_secs(15 * 60);

pub(crate) const SNAKE_NAME: &str = "hovering-hobbs";

#[derive(Debug, Default)]
pub(crate) struct AppState {
    pub game_states: HashMap<String, GameState>,
}

impl AppState {
    fn evict_idle_games(&mut self, now: Instant) {
        self.game_states
            .retain(|_, game_state| now.duration_since(game_state.last_seen) < GAME_STATE_IDLE_TTL);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GameState {
    pub last_move: Option<LastMoveState>,
    pub id_map: HashMap<String, SnakeId>,
    pub last_seen: Instant,
}

#[derive(Debug, Clone)]
pub(crate) struct LastMoveState {
    pub last_return: MinMaxReturn<StandardCellBoard4Snakes11x11, Score>,
    pub last_board: StandardCellBoard4Snakes11x11,
    pub turn: i32,
}

impl GameState {
    pub fn new(id_map: HashMap<String, SnakeId>, now: Instant) -> Self {
        Self {
            last_move: None,
            id_map,
            last_seen: now,
        }
    }
}
pub(crate) async fn route_hobbs_info() -> impl IntoResponse {
    Json(Factory {}.about())
}
// The start/end spans open after the state change so `live_games` exists at
// span creation, the only time Eyes reads span fields.
pub(crate) async fn route_hobbs_start(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let id_map = build_snake_id_map(&game);
    let now = Instant::now();
    let live_games = {
        let mut state = state.lock();
        state.evict_idle_games(now);
        state
            .game_states
            .insert(game.game.id.clone(), GameState::new(id_map, now));
        state.game_states.len()
    };
    let _span = tracing::info_span!(
        "snake.start",
        snake = SNAKE_NAME,
        game_id = %game.game.id,
        turn = game.turn,
        live_games,
    )
    .entered();
    StatusCode::NO_CONTENT
}
pub(crate) async fn route_hobbs_end(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let live_games = {
        let mut state = state.lock();
        state.game_states.remove(&game.game.id);
        state.game_states.len()
    };
    let _span = tracing::info_span!(
        "snake.end",
        snake = SNAKE_NAME,
        game_id = %game.game.id,
        turn = game.turn,
        won = crate::telemetry::won(&game),
        live_games,
    )
    .entered();
    StatusCode::NO_CONTENT
}

#[tracing::instrument(name = "snake.move", skip_all, fields(snake = SNAKE_NAME, game_id = %game.game.id, turn = game.turn))]
pub(crate) async fn route_hobbs_move(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(game): Json<Game>,
) -> Json<MoveOutput> {
    let game_info = game.game.clone();
    let game_id = game_info.id.to_string();
    let turn = game.turn;

    let name = SNAKE_NAME;

    let options: SnakeOptions = SnakeOptions {
        network_latency_padding: Duration::from_millis(150),
        move_ordering: MoveOrdering::BestFirst,
    };

    // A missing state means we never saw /start (e.g. we restarted mid-game)
    // or it was evicted; rebuild it rather than fail the move.
    let game_state = {
        let mut state_guard = state.lock();

        state_guard
            .game_states
            .entry(game_id.clone())
            .or_insert_with(|| GameState::new(build_snake_id_map(&game), Instant::now()))
            .clone()
    };
    let last_move = &game_state.last_move;

    let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &game_state.id_map)
        .expect("TODO: We need to work on our error handling");

    let you_id = game.you_id();

    let initial_return = if let Some(last_move) = last_move
        && last_move.turn == turn - 1
    {
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

        while let Some(moving_snake_id) = current_return.moving_snake_id()
            && let Some(m) = snake_moves.remove(moving_snake_id)
            && let Some(next_return) = current_return.option_for_move(m)
        {
            current_return = next_return.clone();
        }

        while let MinMaxReturn::Node {
            ref options,
            moving_snake_id,
            ..
        } = current_return
            && moving_snake_id == *you_id
        {
            let new_return = options[0].1.clone();
            current_return = new_return;
        }

        Some(current_return)
    } else {
        None
    };

    let my_id = game.you_id();

    // The score is built per move because it needs our length at the root of this search: a leaf
    // longer than that ate somewhere in the window, and counts as fed whatever its health.
    //
    // `initial_return` was scored against last turn's root length. That is safe: the previous tree
    // only orders moves (`MoveOrdering::BestFirst`), and every score in this search is recomputed.
    let root_length = game.get_length_i64(my_id);
    let score = move |board: &StandardCellBoard4Snakes11x11| {
        standard_score_tail_aware::<_, _, 4>(board, root_length)
    };

    let snake = ParanoidMinimaxSnake::new(game, game_info, turn, score, name, options);

    let (_depth, scored) =
        spawn_blocking_with_tracing(move || snake.choose_move_inner(initial_return))
            .await
            .unwrap();

    // A panic here drops the connection, which the proxy turns into a 502, so
    // answer with some move and leave a trace instead.
    let output = scored.your_best_move(my_id).unwrap_or_else(|| {
        tracing::warn!("minimax returned no move for us; falling back to up");
        Move::Up
    });

    // /end can arrive while the move is still computing; don't resurrect it.
    if let Some(game_state) = state.lock().game_states.get_mut(&game_id) {
        game_state.last_move = Some(LastMoveState {
            last_return: scored,
            last_board: game,
            turn,
        });
        game_state.last_seen = Instant::now();
    }

    let output: MoveOutput = MoveOutput {
        r#move: format!("{output}"),
        shout: None,
    };

    Json(output)
}

#[cfg(test)]
mod tests {
    use axum::{Json, extract::State};
    use battlesnake_game_types::wire_representation::Position;

    use super::*;

    fn game(id: &str) -> Game {
        let mut game: Game = serde_json::from_str(include_str!(
            "../../fixtures/130b18e2-8689-4d64-a09f-c4345f80ae79_25.json"
        ))
        .unwrap();
        game.game.id = id.to_string();
        game
    }

    async fn hobbs_move(state: &Arc<Mutex<AppState>>, game: Game) -> String {
        let Json(output) = route_hobbs_move(State(state.clone()), Json(game)).await;
        output.r#move
    }

    fn shared_state() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState::default()))
    }

    #[test]
    fn evicts_only_games_idle_past_the_ttl() {
        let start = Instant::now();
        let mut state = AppState::default();
        for (id, idle) in [
            ("fresh", Duration::ZERO),
            ("just-under", GAME_STATE_IDLE_TTL - Duration::from_secs(1)),
            ("at-ttl", GAME_STATE_IDLE_TTL),
            ("abandoned", GAME_STATE_IDLE_TTL * 4),
        ] {
            let last_seen = start + GAME_STATE_IDLE_TTL * 4 - idle;
            state
                .game_states
                .insert(id.to_string(), GameState::new(HashMap::new(), last_seen));
        }

        state.evict_idle_games(start + GAME_STATE_IDLE_TTL * 4);

        let mut remaining: Vec<_> = state.game_states.keys().map(String::as_str).collect();
        remaining.sort_unstable();
        assert_eq!(remaining, ["fresh", "just-under"]);
    }

    #[tokio::test]
    async fn end_removes_the_game_state() {
        let state = shared_state();

        route_hobbs_start(State(state.clone()), Json(game("a"))).await;
        route_hobbs_start(State(state.clone()), Json(game("b"))).await;
        assert_eq!(state.lock().game_states.len(), 2);

        route_hobbs_end(State(state.clone()), Json(game("a"))).await;
        let state = state.lock();
        assert!(!state.game_states.contains_key("a"));
        assert!(state.game_states.contains_key("b"));
    }

    #[tokio::test]
    async fn move_without_start_builds_state_instead_of_panicking() {
        let state = shared_state();

        let response = route_hobbs_move(State(state.clone()), Json(game("restarted")))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let state = state.lock();
        let game_state = &state.game_states["restarted"];
        assert_eq!(game_state.last_move.as_ref().map(|m| m.turn), Some(25));
    }

    #[tokio::test]
    async fn move_after_end_does_not_resurrect_the_game() {
        let state = shared_state();
        route_hobbs_start(State(state.clone()), Json(game("finished"))).await;

        // Simulate /end landing while the move is computing: the move handler
        // re-creates the state on entry, so remove it again right after.
        let pending = tokio::spawn(route_hobbs_move(
            State(state.clone()),
            Json(game("finished")),
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;
        route_hobbs_end(State(state.clone()), Json(game("finished"))).await;
        pending.await.unwrap();

        assert!(state.lock().game_states.is_empty());
    }

    #[tokio::test]
    async fn arena_health_probe_gets_a_move() {
        // Arena's health probe and "Test Snake" button send a solo turn-0
        // game. This used to panic, and the dropped connection reached
        // Arena as a 502.
        let game: Game =
            serde_json::from_str(include_str!("../../fixtures/arena_health_probe.json")).unwrap();

        let chosen = hobbs_move(&shared_state(), game).await;

        assert!(["up", "down", "left", "right"].contains(&chosen.as_str()));
    }

    #[tokio::test]
    async fn solo_game_avoids_the_walls() {
        let game: Game = serde_json::from_str(include_str!(
            "../../fixtures/solo_cornered_bottom_right.json"
        ))
        .unwrap();

        // Down and Right leave the board, and Up is our own neck.
        assert_eq!(hobbs_move(&shared_state(), game).await, "left");
    }

    #[tokio::test]
    async fn last_snake_standing_gets_a_move() {
        let state = shared_state();
        hobbs_move(&state, game("last-standing")).await;

        // Every opponent died on turn 25. The previous turn's search tree is
        // still reused, and it reaches a snake that no longer exists.
        let mut game = game("last-standing");
        game.turn = 26;
        game.you.body.pop_back();
        game.you.body.push_front(Position { x: 3, y: 9 });
        game.you.head = Position { x: 3, y: 9 };
        game.board.snakes = vec![game.you.clone()];

        // Up is our neck, and also the fallback when the search finds no move.
        assert_ne!(hobbs_move(&state, game).await, "up");
    }
}
