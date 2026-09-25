//! Per-game token counts, estimated cost, latency, and fallback metrics.
//!
//! Set `EYES_ORG_ID`, `EYES_APP_ID`, and `EYES_TOKEN` to publish the
//! `judicious-jev` dashboard. Use a dedicated Eyes app: startup publishes its
//! metric/dashboard manifest. `EYES_URL` defaults to `https://eyes.coreyja.com`.
//! Terrarium's deploy workflow supplies these settings and the TypeSafe key
//! from GitHub; update the repository settings and redeploy to rotate them.
//!
//! Late or invalid answers still count toward usage. Missing usage and unknown
//! model pricing are tracked as unknown, not zero. Costs are estimates; the
//! dashboard displays microUSD (1,000,000 = $1) to keep small amounts visible.

use std::time::Duration;

use battlesnake_rs::Game;
use eyes_subscriber::{
    AggregateFunction, DashboardItem, DashboardSection, NamedDashboard, NamedMetric,
    NamedMetricBuilder,
};

use super::Evaluation;

// https://docs.typesafe.ai/models, checked 2026-09-17. An unknown resolved
// version deliberately has unknown cost until its pricing has been verified.
fn input_rate(model: &str) -> Option<f64> {
    (model == "jev-1.13.0").then_some(0.042)
}

pub(super) fn record_api(
    game: &Game,
    requested_model: &str,
    elapsed: Duration,
    response: Option<&Evaluation>,
) {
    let model = response.map_or(requested_model, |r| r.model.as_str());
    let input_tokens = response.map(|r| r.usage.input_tokens);
    let output_tokens = response.map(|r| r.usage.output_tokens);
    let rate = input_rate(model);
    let cost = input_tokens
        .zip(rate)
        .map(|(tokens, rate)| tokens as f64 * rate / 1_000_000.0);
    tracing::info!(
        event_kind = "jev_api", game_id = game.game.id, snake_id = game.you.id, turn = game.turn,
        request_id = %uuid::Uuid::new_v4(), model, requested_model,
        input_tokens, output_tokens, estimated_cost_usd = cost,
        estimated_cost_microusd = cost.map(|value| value * 1_000_000.0),
        input_usd_per_million = rate, output_usd_per_million = rate.map(|_| 0.0),
        usage_reported = u64::from(response.is_some()), usage_unknown = u64::from(response.is_none()),
        cost_unknown = u64::from(cost.is_none()), api_latency_ms = elapsed.as_secs_f64() * 1000.0,
        "Jev API usage"
    );
}

pub(super) fn record_move(game: &Game, direction: &str, reason: &str, elapsed: Duration) {
    tracing::info!(
        event_kind = "jev_move",
        game_id = game.game.id,
        snake_id = game.you.id,
        turn = game.turn,
        direction,
        reason,
        used_jev = u64::from(reason == "jev"),
        fallback = u64::from(reason != "jev"),
        move_latency_ms = elapsed.as_secs_f64() * 1000.0,
        "Judicious Jev move"
    );
}

fn metric(id: &str, field: &str, event: &str, unit: &str) -> Result<NamedMetricBuilder, String> {
    NamedMetricBuilder::new(id, AggregateFunction::Sum, Some(field))?
        .filter_eq("fields.event_kind", event)
        .map(|builder| builder.unit(unit))
}

/// Jev's metrics and dashboard; `crate::telemetry` publishes them alongside
/// the Terrarium dashboard in the app's single manifest.
pub(crate) fn declarations() -> Result<(Vec<NamedMetric>, NamedDashboard), String> {
    let mut metrics: Vec<NamedMetric> = Vec::new();
    for (id, field, unit, event) in [
        (
            "jev.cost",
            "fields.estimated_cost_microusd",
            "microUSD",
            "jev_api",
        ),
        (
            "jev.input_tokens",
            "fields.input_tokens",
            "tokens",
            "jev_api",
        ),
        (
            "jev.output_tokens",
            "fields.output_tokens",
            "tokens",
            "jev_api",
        ),
        (
            "jev.usage_unknown",
            "fields.usage_unknown",
            "requests",
            "jev_api",
        ),
        (
            "jev.cost_unknown",
            "fields.cost_unknown",
            "requests",
            "jev_api",
        ),
        ("jev.model_moves", "fields.used_jev", "moves", "jev_move"),
        ("jev.fallbacks", "fields.fallback", "moves", "jev_move"),
    ] {
        metrics.push(metric(id, field, event, unit)?.build()?);
        metrics.push(
            metric(&format!("{id}.by_game"), field, event, unit)?
                .group_by("fields.game_id")?
                .group_by("fields.snake_id")?
                .build()?,
        );
    }
    metrics.push(
        metric(
            "jev.cost.over_time",
            "fields.estimated_cost_microusd",
            "jev_api",
            "microUSD",
        )?
        .time_bucket(300)
        .build()?,
    );
    metrics.push(
        NamedMetricBuilder::new(
            "jev.move_latency_p95",
            AggregateFunction::P95,
            Some("fields.move_latency_ms"),
        )?
        .filter_eq("fields.event_kind", "jev_move")?
        .unit("ms")
        .build()?,
    );
    metrics.push(
        NamedMetricBuilder::new(
            "jev.api_latency_p95",
            AggregateFunction::P95,
            Some("fields.api_latency_ms"),
        )?
        .filter_eq("fields.event_kind", "jev_api")?
        .unit("ms")
        .build()?,
    );
    let dashboard = NamedDashboard::new("judicious-jev", "Judicious Jev")?
        .description("Jev usage, estimated cost, and move deadlines. Costs are in microUSD: 1,000,000 microUSD = $1. Cost covers responses with known pricing. Unknown usage may still be billable; late responses are included. Per-game totals cover the selected time range.")
        .default_range_seconds(86400)
        .section(DashboardSection::new().title("Usage and cost")
            .item(DashboardItem::stat("jev.cost").label("Reported cost (microUSD)"))
            .item(DashboardItem::stat("jev.input_tokens").label("Input tokens"))
            .item(DashboardItem::stat("jev.output_tokens").label("Output tokens"))
            .item(DashboardItem::stat("jev.usage_unknown").label("Requests with unknown usage"))
            .item(DashboardItem::stat("jev.cost_unknown").label("Requests with unknown cost"))
            .item(DashboardItem::time_series("jev.cost.over_time").label("Cost over time")))
        .section(DashboardSection::new().title("Decisions and latency")
            .item(DashboardItem::stat("jev.model_moves").label("Moves chosen by Jev"))
            .item(DashboardItem::stat("jev.fallbacks").label("Local moves"))
            .item(DashboardItem::stat("jev.move_latency_p95").label("Move latency p95"))
            .item(DashboardItem::stat("jev.api_latency_p95").label("API latency p95")))
        .section(DashboardSection::new().title("Per game and snake")
            .item(DashboardItem::table("jev.cost.by_game").label("Cost (microUSD)"))
            .item(DashboardItem::table("jev.input_tokens.by_game").label("Input tokens"))
            .item(DashboardItem::table("jev.output_tokens.by_game").label("Output tokens"))
            .item(DashboardItem::table("jev.fallbacks.by_game").label("Local moves"))
            .item(DashboardItem::table("jev.cost_unknown.by_game").label("Requests with unknown cost")));
    Ok((metrics, dashboard))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_declarations_validate() {
        let (metrics, dashboard) = declarations().unwrap();
        assert_eq!(metrics.len(), 17);
        assert_eq!(dashboard.id, "judicious-jev");
    }

    #[test]
    fn unknown_model_does_not_inherit_old_pricing() {
        assert_eq!(input_rate("jev-1.13.0"), Some(0.042));
        assert_eq!(input_rate("jev-2.0.0"), None);
    }
}
