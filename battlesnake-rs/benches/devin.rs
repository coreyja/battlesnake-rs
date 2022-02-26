use battlesnake_game_types::{
    compact_representation::StandardCellBoard4Snakes11x11, types::build_snake_id_map,
    wire_representation::Game,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

fn bench_minmax_to_turn(c: &mut Criterion, max_turns: usize) {
    let game_json = include_str!("../fixtures/start_of_game.json");

    let mut group = c.benchmark_group(format!("Devin: Turns {}", max_turns));

    // group.bench_function("compact full-minmax", |b| {
    //     b.iter(|| {
    //         let game_state: Game = serde_json::from_str(game_json).unwrap();
    //         let id_map = build_snake_id_map(&game_state);
    //         let game_state =
    //             StandardCellBoard4Snakes11x11::convert_from_game(game_state, &id_map).unwrap();
    //         minmax_bench_entry(black_box(game_state), max_turns)
    //     })
    // });

    // group.bench_function("compact full-minmax iterative deepened", |b| {
    //     b.iter(|| {
    //         let game_state: Game = serde_json::from_str(game_json).unwrap();
    //         let id_map = build_snake_id_map(&game_state);
    //         let game_state =
    //             StandardCellBoard4Snakes11x11::convert_from_game(game_state, &id_map).unwrap();
    //         minmax_deepened_bench_entry(black_box(game_state), max_turns)
    //     })
    // });

    group.bench_function("compact eval-minmax", |b| {
        b.iter(|| {
            let game_state: Game = serde_json::from_str(game_json).unwrap();
            let devin = battlesnake_rs::devious_devin_eval::Factory::new().create(game_state);
            devin.single_minimax_bench(max_turns)
        })
    });

    group.bench_function("compact eval-minmax iterative deepened", |b| {
        b.iter(|| {
            let game_state: Game = serde_json::from_str(game_json).unwrap();
            let devin = battlesnake_rs::devious_devin_eval::Factory::new().create(game_state);
            devin.deepend_minimax_bench(max_turns)
        })
    });

    // group.bench_function("wire partial-minmax", |b| {
    //     b.iter(|| {
    //         let game: Game = serde_json::from_str(game_json).unwrap();
    //         battlesnake_rs::devious_devin_mutable::minmax_bench_entry(black_box(game), max_turns)
    //     })
    // });

    // group.bench_function("wire partial-minmax iterative deepened", |b| {
    //     b.iter(|| {
    //         let game: Game = serde_json::from_str(game_json).unwrap();
    //         battlesnake_rs::devious_devin_mutable::minmax_deepened_bench_entry(
    //             black_box(game),
    //             max_turns,
    //         )
    //     })
    // });

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
