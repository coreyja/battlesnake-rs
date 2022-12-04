use battlesnake_minimax::paranoid::Scorable;
use types::{
    compact_representation::StandardCellBoard4Snakes11x11, types::build_snake_id_map,
    wire_representation::Game,
};

use battlesnake_rs::{
    devious_devin_eval::{score, ScoreEndState},
    MinimaxSnake,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

fn create_snake(
    game: Game,
) -> MinimaxSnake<
    StandardCellBoard4Snakes11x11,
    ScoreEndState,
    impl Scorable<StandardCellBoard4Snakes11x11, ScoreEndState> + Clone,
    4,
> {
    let game_info = game.game.clone();
    let turn = game.turn;
    let id_map = build_snake_id_map(&game);

    let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

    MinimaxSnake::from_fn(game, game_info, turn, &score, "devin")
}

fn bench_minmax_to_turn(c: &mut Criterion, max_turns: usize) {
    let game_json = include_str!("../fixtures/start_of_game.json");

    let mut group = c.benchmark_group(format!("Devin: Turns {max_turns}"));

    group.bench_function("compact eval-minmax", |b| {
        b.iter(|| {
            let game_state: Game = serde_json::from_str(game_json).unwrap();
            let devin = create_snake(black_box(game_state));
            devin.single_minimax(max_turns)
        })
    });

    group.bench_function("compact eval-minmax iterative deepened", |b| {
        b.iter(|| {
            let game_state: Game = serde_json::from_str(game_json).unwrap();
            let devin = create_snake(black_box(game_state));
            devin.deepend_minimax_to_turn(max_turns)
        })
    });

    // group.bench_function("wire eval-minmax", |b| {
    //     b.iter(|| {
    //         let game_state: Game = serde_json::from_str(game_json).unwrap();
    //         let devin = {
    //             let game = black_box(game_state);
    //             let game_info = game.game.clone();
    //             let turn = game.turn;
    //             let id_map = build_snake_id_map(&game);

    //             MinimaxSnake::from_fn(game, game_info, turn, &score, "devin")
    //         };
    //         devin.single_minimax(max_turns)
    //     })
    // });

    // group.bench_function("wire eval-minmax iterative deepened", |b| {
    //     b.iter(|| {
    //         let game_state: Game = serde_json::from_str(game_json).unwrap();
    //         let devin = {
    //             let game = black_box(game_state);
    //             let game_info = game.game.clone();
    //             let turn = game.turn;
    //             let id_map = build_snake_id_map(&game);

    //             MinimaxSnake::from_fn(game, game_info, turn, &score, "devin")
    //         };
    //         devin.deepend_minimax_to_turn(max_turns)
    //     })
    // });

    group.finish();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    bench_minmax_to_turn(c, 3);
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
