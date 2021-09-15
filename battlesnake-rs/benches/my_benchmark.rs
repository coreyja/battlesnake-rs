use battlesnake_rs::devious_devin::{minmax_bench_entry, minmax_deepened_bench_entry};
use battlesnake_rs::GameState;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

fn bench_minmax_to_turn(c: &mut Criterion, max_turns: usize) {
    let game_json = include_str!("../fixtures/start_of_game.json");

    let mut group = c.benchmark_group(format!("Devin: Turns {}", max_turns));

    group.bench_function("wire minmax", |b| {
        b.iter(|| {
            let game: GameState = serde_json::from_str(game_json).unwrap();
            minmax_bench_entry(black_box(game), max_turns)
        })
    });

    group.bench_function("wire Iterative Deepend with last state", |b| {
        b.iter(|| {
            let game: GameState = serde_json::from_str(game_json).unwrap();
            minmax_deepened_bench_entry(black_box(game), max_turns)
        })
    });

    group.finish();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    bench_minmax_to_turn(c, 4);
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
