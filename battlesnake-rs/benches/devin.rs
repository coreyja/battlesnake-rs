use battlesnake_game_types::{
    compact_representation::CellBoard, types::build_snake_id_map, wire_representation::Game,
};
use battlesnake_rs::devious_devin_full::{minmax_bench_entry, minmax_deepened_bench_entry};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

fn bench_minmax_to_turn(c: &mut Criterion, max_turns: usize) {
    let game_json = include_str!("../fixtures/start_of_game.json");

    let mut group = c.benchmark_group(format!("Devin: Turns {}", max_turns));

    group.bench_function("wire full-minmax", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            minmax_bench_entry(black_box(game), max_turns)
        })
    });

    group.bench_function("wire full-minmax iterative deepened", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            minmax_deepened_bench_entry(black_box(game), max_turns)
        })
    });

    group.bench_function("compact full-minmax", |b| {
        b.iter(|| {
            let game_state: Game = serde_json::from_str(game_json).unwrap();
            let id_map = build_snake_id_map(&game_state);
            let game_state: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11 =
                CellBoard::convert_from_game(game_state, &id_map).unwrap();
            minmax_bench_entry(black_box(game_state), max_turns)
        })
    });

    group.bench_function("compact full-minmax iterative deepened", |b| {
        b.iter(|| {
            let game_state: Game = serde_json::from_str(game_json).unwrap();
            let id_map = build_snake_id_map(&game_state);
            let game_state: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11 =
                CellBoard::convert_from_game(game_state, &id_map).unwrap();
            minmax_deepened_bench_entry(black_box(game_state), max_turns)
        })
    });

    group.bench_function("wire partial-minmax", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            battlesnake_rs::devious_devin_mutative::minmax_bench_entry(black_box(game), max_turns)
        })
    });

    group.bench_function("wire partial-minmax iterative deepened", |b| {
        b.iter(|| {
            let game: Game = serde_json::from_str(game_json).unwrap();
            battlesnake_rs::devious_devin_mutative::minmax_deepened_bench_entry(
                black_box(game),
                max_turns,
            )
        })
    });

    // group.bench_function("compact partial-minmax", |b| {
    //     b.iter(|| {
    //         let game_state: Game = serde_json::from_str(game_json).unwrap();
    //         let id_map = build_snake_id_map(&game_state);
    //         let game_state: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11 =
    //             CellBoard::convert_from_game(game_state, &id_map).unwrap();
    //         minmax_bench_entry(black_box(game_state), max_turns)
    //     })
    // });

    // group.bench_function("compact partial-minmax iterative deepened", |b| {
    //     b.iter(|| {
    //         let game_state: Game = serde_json::from_str(game_json).unwrap();
    //         let id_map = build_snake_id_map(&game_state);
    //         let game_state: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11 =
    //             CellBoard::convert_from_game(game_state, &id_map).unwrap();
    //         minmax_deepened_bench_entry(black_box(game_state), max_turns)
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
