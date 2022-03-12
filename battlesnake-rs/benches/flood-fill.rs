use battlesnake_game_types::wire_representation::Game;
use battlesnake_game_types::{types::*, wire_representation::Ruleset};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("Flood Fill");

    g.bench_function("compact spread", |b| {
        use battlesnake_rs::flood_fill::spread_from_head::SpreadFromHead;

        let game_json = include_str!("../fixtures/start_of_game.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        let id_map = build_snake_id_map(&game);
        let game = battlesnake_game_types::compact_representation::StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.squares_per_snake(5)
        })
    });

    g.bench_function("wrapped spread", |b| {
        use battlesnake_rs::flood_fill::spread_from_head::SpreadFromHead;

        let game_json = include_str!("../fixtures/start_of_game.json");
        let mut game: Game = serde_json::from_str(game_json).unwrap();
        game.game.ruleset = Ruleset { name: "wrapped".to_string(), version: "1.0".to_string(), settings: None };

        let id_map = build_snake_id_map(&game);
        let game = battlesnake_game_types::compact_representation::WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.squares_per_snake(5)
        })
    });

    g.bench_function("wrapped jump", |b| {
        use battlesnake_rs::flood_fill::jump_flooding::JumpFlooding;

        let game_json = include_str!("../fixtures/start_of_game.json");
        let mut game: Game = serde_json::from_str(game_json).unwrap();
        game.game.ruleset = Ruleset { name: "wrapped".to_string(), version: "1.0".to_string(), settings: None };

        let id_map = build_snake_id_map(&game);
        let game = battlesnake_game_types::compact_representation::WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.squares_per_snake()
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
