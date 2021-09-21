use battlesnake_game_types::types::*;

use battlesnake_rs::a_prime::APrimeCalculable;
use battlesnake_rs::*;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("a-prime start_of_game", |b| {
        let game_json = include_str!("../fixtures/start_of_game.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.shortest_path(&game.you.head, &game.board.food, None)
        })
    });

    c.bench_function("a-prime a-prime-food-maze", |b| {
        let game_json = include_str!("../fixtures/a-prime-food-maze.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.shortest_path(&game.you.head, &game.board.food, None)
        })
    });

    c.bench_function("compact a-prime start_of_game", |b| {
        let game_json = include_str!("../fixtures/start_of_game.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        let id_map = build_snake_id_map(&game);
        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.shortest_path(
                &game.get_head_as_native_position(game.you_id()),
                &game.get_all_food_as_native_positions(),
                None,
            )
        })
    });

    c.bench_function("compact a-prime a-prime-food-maze", |b| {
        let game_json = include_str!("../fixtures/a-prime-food-maze.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        let id_map = build_snake_id_map(&game);
        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.shortest_path(
                &game.get_head_as_native_position(game.you_id()),
                &game.get_all_food_as_native_positions(),
                None,
            )
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
