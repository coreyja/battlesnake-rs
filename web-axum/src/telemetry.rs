//! Eyes wiring for the whole server: the tracing layer, what reaches Eyes, and
//! the boot manifest carrying the Terrarium and Judicious Jev dashboards.
//!
//! Dashboard metrics query spans, and Eyes only sees span fields that exist
//! when a span is created (`Span::record` afterwards is invisible to queries).
//! Every snake's handlers therefore open `snake.start` / `snake.move` /
//! `snake.end` spans that already carry what the dashboard groups and
//! aggregates on: `snake`, `turn`, and on `snake.end`, `won`.

use std::time::Duration;

use battlesnake_rs::Game;
use color_eyre::eyre::{Result, eyre};
use eyes_subscriber::{
    AggregateFunction, AppManifest, DashboardItem, DashboardSection, EyesLayer, EyesShutdownHandle,
    EyesSubscriberBuilder, NamedDashboard, NamedMetric, NamedMetricBuilder, TransportType,
};
use tracing::{Level, Metadata};
use tracing_subscriber::filter::{FilterFn, filter_fn};

use crate::judicious_jev;

pub(crate) const START_SPAN: &str = "snake.start";
pub(crate) const MOVE_SPAN: &str = "snake.move";
pub(crate) const END_SPAN: &str = "snake.end";

pub(crate) fn layer() -> Result<(Option<EyesLayer>, Option<EyesShutdownHandle>)> {
    match (
        std::env::var("EYES_ORG_ID").ok(),
        std::env::var("EYES_APP_ID").ok(),
    ) {
        (None, None) => Ok((None, None)),
        (Some(org), Some(app)) => {
            let (layer, shutdown) = EyesSubscriberBuilder::from_env(org.parse()?, app.parse()?)?
                .with_queue_capacity(4096)
                .build_with_transport(TransportType::BatchingHttp);
            Ok((Some(layer), Some(shutdown)))
        }
        _ => Err(eyre!("Set both EYES_ORG_ID and EYES_APP_ID to enable Eyes")),
    }
}

/// Eyes gets every INFO+ span but only the events that carry signal: WARN+
/// (including panics) and Judicious Jev's metric events. Per-request log lines
/// stay in the journal.
pub(crate) fn eyes_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    filter_fn(|metadata| {
        if metadata.is_span() {
            *metadata.level() <= Level::INFO
        } else {
            *metadata.level() <= Level::WARN
                || metadata.target().starts_with("web_axum::judicious_jev")
        }
    })
}

/// 1 when `you` is the last snake standing at `/end`, else 0. Numeric so the
/// dashboard can average it into a win rate.
pub(crate) fn won(game: &Game) -> u64 {
    u64::from(game.board.snakes.len() == 1 && game.board.snakes[0].id == game.you.id)
}

pub(crate) fn publish() -> Result<()> {
    let manifest = manifest().map_err(|error| eyre!(error))?;
    tokio::spawn(async move {
        match tokio::time::timeout(
            Duration::from_secs(10),
            eyes_subscriber::send_manifest_from_env(&manifest),
        )
        .await
        {
            Ok(Ok(())) => tracing::info!("Published Eyes dashboards"),
            Ok(Err(error)) => tracing::warn!(%error, "Could not publish Eyes dashboards"),
            Err(_) => tracing::warn!("Timed out publishing Eyes dashboards"),
        }
    });
    Ok(())
}

/// One manifest per Eyes app: publishing replaces the app's previous
/// declarations, so every dashboard has to ship together.
fn manifest() -> Result<AppManifest, String> {
    let (mut metrics, jev_dashboard) = judicious_jev::telemetry::declarations()?;
    let (terrarium_metrics, terrarium_dashboard) = terrarium_declarations()?;
    metrics.extend(terrarium_metrics);
    Ok(AppManifest::default()
        .app_version(env!("CARGO_PKG_VERSION"))
        .metrics(metrics)
        .dashboards(vec![terrarium_dashboard, jev_dashboard]))
}

fn on_span(
    id: &str,
    span: &str,
    function: AggregateFunction,
    field: Option<&str>,
) -> Result<NamedMetricBuilder, String> {
    NamedMetricBuilder::new(id, function, field)?.filter_eq("name", span)
}

fn terrarium_declarations() -> Result<(Vec<NamedMetric>, NamedDashboard), String> {
    use AggregateFunction::{Avg, Count, Max, P50, P95, Sum};

    let every_5_minutes = 300;
    let metrics = vec![
        // Traffic
        on_span("terrarium.moves", MOVE_SPAN, Count, None)?
            .unit("moves")
            .build()?,
        on_span("terrarium.moves.over_time", MOVE_SPAN, Count, None)?
            .unit("moves")
            .time_bucket(every_5_minutes)
            .build()?,
        on_span("terrarium.moves.by_snake", MOVE_SPAN, Count, None)?
            .unit("moves")
            .group_by("fields.snake")?
            .build()?,
        on_span("terrarium.games_started", START_SPAN, Count, None)?
            .unit("games")
            .build()?,
        on_span("terrarium.games_started.by_snake", START_SPAN, Count, None)?
            .unit("games")
            .group_by("fields.snake")?
            .build()?,
        // Move speed
        on_span("terrarium.move_p50", MOVE_SPAN, P50, Some("duration"))?.build()?,
        on_span("terrarium.move_p95", MOVE_SPAN, P95, Some("duration"))?.build()?,
        on_span("terrarium.slowest_move", MOVE_SPAN, Max, Some("duration"))?.build()?,
        on_span(
            "terrarium.move_p95.over_time",
            MOVE_SPAN,
            P95,
            Some("duration"),
        )?
        .time_bucket(every_5_minutes)
        .build()?,
        on_span(
            "terrarium.move_p95.by_snake",
            MOVE_SPAN,
            P95,
            Some("duration"),
        )?
        .group_by("fields.snake")?
        .build()?,
        // Results
        on_span(
            "terrarium.win_rate.by_snake",
            END_SPAN,
            Avg,
            Some("fields.won"),
        )?
        .filter_numeric("fields.won")?
        .group_by("fields.snake")?
        .build()?,
        on_span("terrarium.wins.by_snake", END_SPAN, Sum, Some("fields.won"))?
            .filter_numeric("fields.won")?
            .unit("games")
            .group_by("fields.snake")?
            .build()?,
        on_span("terrarium.games_finished.by_snake", END_SPAN, Count, None)?
            .unit("games")
            .group_by("fields.snake")?
            .build()?,
        on_span("terrarium.longest_game", END_SPAN, Max, Some("fields.turn"))?
            .filter_numeric("fields.turn")?
            .unit("turns")
            .build()?,
        on_span(
            "terrarium.game_length.by_snake",
            END_SPAN,
            Avg,
            Some("fields.turn"),
        )?
        .filter_numeric("fields.turn")?
        .unit("turns")
        .group_by("fields.snake")?
        .build()?,
        // Health
        NamedMetricBuilder::new("terrarium.panics", Count, None)?
            .filter_eq("message", "panic")?
            .unit("panics")
            .build()?,
        NamedMetricBuilder::new("terrarium.panics.by_location", Count, None)?
            .filter_eq("message", "panic")?
            .unit("panics")
            .group_by("fields.panic_location")?
            .build()?,
        NamedMetricBuilder::new("terrarium.hobbs_live_games", Max, Some("fields.live_games"))?
            .filter_eq("fields.snake", crate::hobbs::SNAKE_NAME)?
            .filter_numeric("fields.live_games")?
            .unit("games")
            .build()?,
        NamedMetricBuilder::new(
            "terrarium.hobbs_live_games.over_time",
            Max,
            Some("fields.live_games"),
        )?
        .filter_eq("fields.snake", crate::hobbs::SNAKE_NAME)?
        .filter_numeric("fields.live_games")?
        .unit("games")
        .time_bucket(every_5_minutes)
        .build()?,
    ];

    let dashboard = NamedDashboard::new("terrarium", "Terrarium")?
        .description("Every snake served by terrarium.coreyja.com: traffic, move speed, results, and server health. Built from the snake.start / snake.move / snake.end spans. Win rate is 0–1: the share of finished games where the snake was the last one standing.")
        .default_range_seconds(86400)
        .section(DashboardSection::new().title("Traffic")
            .item(DashboardItem::stat("terrarium.moves").label("Moves"))
            .item(DashboardItem::stat("terrarium.games_started").label("Games started"))
            .item(DashboardItem::time_series("terrarium.moves.over_time").label("Moves over time"))
            .item(DashboardItem::table("terrarium.moves.by_snake").label("Busiest snakes"))
            .item(DashboardItem::table("terrarium.games_started.by_snake").label("Games started by snake")))
        .section(DashboardSection::new().title("Move speed")
            .item(DashboardItem::stat("terrarium.move_p50").label("Move time p50"))
            .item(DashboardItem::stat("terrarium.move_p95").label("Move time p95"))
            .item(DashboardItem::stat("terrarium.slowest_move").label("Slowest move"))
            .item(DashboardItem::time_series("terrarium.move_p95.over_time").label("Move time p95 over time"))
            .item(DashboardItem::table("terrarium.move_p95.by_snake").label("Move time p95 by snake")))
        .section(DashboardSection::new().title("Results")
            .item(DashboardItem::stat("terrarium.longest_game").label("Longest game"))
            .item(DashboardItem::table("terrarium.win_rate.by_snake").label("Win rate by snake (0–1)"))
            .item(DashboardItem::table("terrarium.wins.by_snake").label("Wins by snake"))
            .item(DashboardItem::table("terrarium.games_finished.by_snake").label("Games finished by snake"))
            .item(DashboardItem::table("terrarium.game_length.by_snake").label("Average game length by snake")))
        .section(DashboardSection::new().title("Health")
            .item(DashboardItem::stat("terrarium.panics").label("Panics"))
            .item(DashboardItem::table("terrarium.panics.by_location").label("Panics by location"))
            .item(DashboardItem::stat("terrarium.hobbs_live_games").label("Peak Hovering Hobbs games in memory"))
            .item(DashboardItem::time_series("terrarium.hobbs_live_games.over_time").label("Hovering Hobbs games in memory")));

    Ok((metrics, dashboard))
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use eyes_subscriber::MetricResultShape;

    use super::*;

    #[test]
    fn every_dashboard_item_references_a_declared_metric_of_matching_shape() {
        let manifest = manifest().unwrap();
        let metrics = manifest.metrics.as_ref().unwrap();

        let shapes: HashMap<&str, &MetricResultShape> = metrics
            .iter()
            .map(|metric| (metric.id.as_str(), &metric.result_shape))
            .collect();
        assert_eq!(shapes.len(), metrics.len(), "metric ids must be unique");

        let dashboards = manifest.dashboards.as_ref().unwrap();
        let ids: HashSet<&str> = dashboards.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, HashSet::from(["terrarium", "judicious-jev"]));

        for item in dashboards
            .iter()
            .flat_map(|d| &d.sections)
            .flat_map(|s| &s.items)
        {
            let (query_id, expected) = match item {
                DashboardItem::Stat { query_id, .. } => (query_id, MetricResultShape::Scalar),
                DashboardItem::Table { query_id, .. } => (query_id, MetricResultShape::Table),
                DashboardItem::TimeSeries { query_id, .. } => {
                    (query_id, MetricResultShape::TimeSeries)
                }
                _ => continue,
            };
            let shape = shapes
                .get(query_id.as_str())
                .unwrap_or_else(|| panic!("{query_id} is not a declared metric"));
            assert_eq!(**shape, expected, "{query_id} has the wrong result shape");
        }
    }

    #[test]
    fn won_only_when_you_are_the_last_snake_standing() {
        let mut game: Game = serde_json::from_str(include_str!(
            "../../fixtures/130b18e2-8689-4d64-a09f-c4345f80ae79_25.json"
        ))
        .unwrap();
        assert_eq!(won(&game), 0, "four snakes still alive");

        let you = game.you.id.clone();
        game.board.snakes.retain(|snake| snake.id == you);
        assert_eq!(won(&game), 1);

        game.board.snakes.clear();
        assert_eq!(won(&game), 0, "a draw is not a win");
    }
}
