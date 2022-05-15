use battlesnake_rs::{mcts_snake::MctsSnake, StandardCellBoard4Snakes11x11};

use battlesnake_game_types::{
    compact_representation::WrappedCellBoard4Snakes11x11,
    types::build_snake_id_map,
    wire_representation::{Game, Ruleset},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("MCTS");
    let game_json = include_str!("../fixtures/start_of_game.json");

    g.bench_function("MCTS Compact", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            let game_info = game.game.clone();
            let id_map = build_snake_id_map(&game);

            let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(black_box(game), game_info);

            snake.mcts_bench(3)
        })
    });

    g.bench_function("Hobbs Wrapped", |b| {
        b.iter(|| {
            let mut game: Game = serde_json::from_str(game_json).unwrap();
            game.game.ruleset = Ruleset {
                name: "wrapped".to_string(),
                version: "1.0".to_string(),
                settings: None,
            };
            let game_info = game.game.clone();
            let id_map = build_snake_id_map(&game);

            let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let snake = MctsSnake::new(black_box(game), game_info);

            snake.mcts_bench(3)
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
