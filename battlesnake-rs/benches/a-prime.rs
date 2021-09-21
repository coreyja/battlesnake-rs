use battlesnake_rs::a_prime::shortest_path;
use battlesnake_rs::GameState;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("a-prime start_of_game", |b| {
        let game_json = include_str!("../fixtures/start_of_game.json");

        b.iter(|| {
            let game: GameState = serde_json::from_str(game_json).unwrap();
            let game = black_box(game);
            shortest_path(&game.board, &game.you.head, &game.board.food, None)
        })
    });

    c.bench_function("a-prime a-prime-food-maze", |b| {
        let game_json = include_str!("../fixtures/a-prime-food-maze.json");

        b.iter(|| {
            let game: GameState = serde_json::from_str(game_json).unwrap();
            let game = black_box(game);
            shortest_path(&game.board, &game.you.head, &game.board.food, None)
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
