use battlesnake_minimax::paranoid::CachedScore;
use battlesnake_rs::{MinimaxSnake, StandardCellBoard4Snakes11x11};

use battlesnake_game_types::{
    compact_representation::{
        dimensions::ArcadeMaze, WrappedCellBoard, WrappedCellBoard4Snakes11x11,
    },
    types::build_snake_id_map,
    wire_representation::{Game, Ruleset},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

use battlesnake_rs::hovering_hobbs::standard_score;

pub fn criterion_benchmark(c: &mut Criterion) {
    {
        let mut g = c.benchmark_group("Hobbs/fixture: start_of_game.json");
        let game_json = include_str!("../fixtures/start_of_game.json");

        g.bench_function("Compact", |b| {
            b.iter(|| {
                let game: Game = serde_json::from_str(game_json).unwrap();
                let game_info = game.game.clone();
                let turn = game.turn;
                let id_map = build_snake_id_map(&game);

                let name = "hovering-hobbs";
                let score_map = Default::default();
                let cached_score = CachedScore::new(&standard_score::<_, _, 4>, score_map);

                let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

                let snake = MinimaxSnake::new(
                    black_box(game),
                    game_info,
                    turn,
                    cached_score,
                    name,
                    Default::default(),
                );

                snake.deepend_minimax_to_turn(3)
            })
        });

        g.bench_function("Wrapped", |b| {
            b.iter(|| {
                let mut game: Game = serde_json::from_str(game_json).unwrap();
                game.game.ruleset = Ruleset {
                    name: "wrapped".to_string(),
                    version: "1.0".to_string(),
                    settings: None,
                };
                let game_info = game.game.clone();
                let turn = game.turn;
                let id_map = build_snake_id_map(&game);

                let name = "hovering-hobbs";
                let score_map = Default::default();
                let cached_score = CachedScore::new(&standard_score::<_, _, 4>, score_map);

                let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

                let snake = MinimaxSnake::new(
                    black_box(game),
                    game_info,
                    turn,
                    cached_score,
                    name,
                    Default::default(),
                );

                snake.deepend_minimax_to_turn(3)
            });
        });
    }

    {
        let mut g = c.benchmark_group("Hobbs/fixture: arcade_maze_end_game_duels.json");
        let game_json = include_str!("../../fixtures/arcade_maze_end_game_duels.json");

        g.bench_function("arcade-maze", |b| {
            b.iter(|| {
                let mut game: Game = serde_json::from_str(game_json).unwrap();
                game.game.ruleset = Ruleset {
                    name: "wrapped".to_string(),
                    version: "1.0".to_string(),
                    settings: None,
                };
                let game_info = game.game.clone();
                let turn = game.turn;
                let id_map = build_snake_id_map(&game);

                let name = "hovering-hobbs";
                let score_map = Default::default();
                let cached_score = CachedScore::new(&standard_score::<_, _, 4>, score_map);

                let game = WrappedCellBoard::<u16, ArcadeMaze, { 19 * 21 }, 4>::convert_from_game(
                    game, &id_map,
                )
                .unwrap();

                let snake = MinimaxSnake::new(
                    black_box(game),
                    game_info,
                    turn,
                    cached_score,
                    name,
                    Default::default(),
                );

                snake.deepend_minimax_to_turn(6)
            });
        });
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
