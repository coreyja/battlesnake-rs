use battlesnake_game_types::{
    compact_representation::CellBoard4Snakes11x11, wire_representation::Game,
};
use battlesnake_rs::devious_devin::{
    minmax_bench_entry, minmax_deepened_bench_entry, minmax_deepened_bench_entry_no_ordering,
    DeviousDevin,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

fn bench_minmax_to_depth(c: &mut Criterion, max_depth: usize) {
    let game_json = include_str!("../fixtures/start_of_game.json");

    let mut group = c.benchmark_group(format!("Devin Depth {}", max_depth));

    group.bench_function("minmax", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            minmax_bench_entry(black_box(game), max_depth)
        })
    });

    group.bench_function("Iterative Deepend with last state ", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            minmax_deepened_bench_entry(black_box(game), max_depth)
        })
    });

    group.bench_function("Iterative deepened with no move reordering", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            minmax_deepened_bench_entry_no_ordering(black_box(game), max_depth)
        })
    });

    group.finish();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    bench_minmax_to_depth(c, 2);
    bench_minmax_to_depth(c, 4);
    bench_minmax_to_depth(c, 6);
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
