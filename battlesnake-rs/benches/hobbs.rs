use battlesnake_rs::{EvalMinimaxSnake, StandardCellBoard4Snakes11x11};

use battlesnake_game_types::{
    compact_representation::WrappedCellBoard4Snakes11x11, types::build_snake_id_map,
    wire_representation::Game,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

use battlesnake_rs::hovering_hobbs::score;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("Hobbs");
    let game_json = include_str!("../fixtures/start_of_game.json");

    g.bench_function("Hobbs Compact", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            let game_info = game.game.clone();
            let turn = game.turn;
            let id_map = build_snake_id_map(&game);

            let name = "hovering-hobbs";

            let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = EvalMinimaxSnake::new(black_box(game), game_info, turn, &score, name);

            snake.deepend_minimax_bench(3)
        })
    });

    g.bench_function("Hobbs Wrapped", |b| {
        b.iter(|| {
            let mut game: Game = serde_json::from_str(game_json).unwrap();
            game.game.ruleset = "wrapped".to_string();
            let game_info = game.game.clone();
            let turn = game.turn;
            let id_map = build_snake_id_map(&game);

            let name = "hovering-hobbs";

            let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = EvalMinimaxSnake::new(black_box(game), game_info, turn, &score, name);

            snake.deepend_minimax_bench(3)
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
