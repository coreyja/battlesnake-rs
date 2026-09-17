# Judicious Jev

Jev is the J in the named snake lineup, after Improbable Irene. It uses
[TypeSafe's Choice API](https://docs.typesafe.ai/primitives/choice) to choose its next move.

## Run

Set `TYPESAFE_API_KEY` through your environment or secret manager, then:

```sh
cargo run -p web-axum
```

The snake URL is `http://localhost:3000/judicious-jev`. Set `PORT` to change the port.
`TYPESAFE_MODEL` defaults to `jev-latest`. Without an API key, the server logs a
warning and Jev uses its local fallback. `GET /judicious-jev` returns Battlesnake v1
metadata; `POST /judicious-jev/start`, `/move`, and `/end` implement the game lifecycle.

## Decisions and deadlines

Rust removes wall, body, neck, starvation, and lethal hazard moves. It accounts for
moving/stacked tails, wrapped edges, and constrictor growth. If any move avoids a
possible losing head-to-head, only those moves are offered. It also computes
reachable space, food distance, post-move health, and cramped regions. These are
static board heuristics, not a search or a guarantee of future survival. Squad
special rules and unusual custom rulesets are not modeled.

One async HTTP request asks Jev to choose from the remaining directions. The prompt
is in `web-axum/src/judicious_jev/instructions.txt`; opponent names and shouts are
excluded. A forced move makes no API call. Returned choices must be in the offered
set. The local fallback prioritizes surviving head-to-heads, room, and food when hungry.

The move handler reserves 50 ms of `game.timeout` for transport and overhead, and
waits at most 450 ms for Jev, including its local preparation time in the turn budget.
API errors, exhausted concurrency, or late responses return the local move. There
are no retries. One shared reqwest client reuses HTTP connections.

An API request that misses the move deadline continues in the background, for at
most five seconds, to capture billable usage. A 32-request semaphore bounds this
work across concurrent games. The late answer never changes an already returned move.

## Eyes dashboard

Set all of the following to export Jev's events through the bounded, batching Eyes
subscriber and publish its named metrics/dashboard at startup:

```sh
export EYES_ORG_ID='<organization UUID>'
export EYES_APP_ID='<dedicated Judicious Jev app UUID>'
export EYES_TOKEN='<app-scoped ingest token>'
# EYES_URL defaults to https://eyes.coreyja.com
```

Use a dedicated Eyes app: the boot manifest publishes that app's complete named
metric and dashboard declarations. Only `web_axum::judicious_jev` events are exported.
Network delivery and manifest publication run outside the move handler. Local
structured logs remain available even when Eyes is disabled or unreachable.

The dashboard appears at
`/orgs/<org>/apps/<app>/dashboards/judicious-jev` and includes:

- Input/output token totals and estimated spend, overall and grouped by game and snake.
- Requests with unknown usage or unverified model pricing.
- Cost over time, Jev decisions, local fallback counts, and move/API p95 latency.

Every `jev_api` event includes game ID, snake ID, turn, request ID, requested/resolved
model, reported tokens, latency, and pricing. Every `jev_move` event records the
returned direction, decision source, and handler latency. Start/end events provide
lifecycle markers. An API error without usage is recorded as unknown, not free.
Invalid choices with valid usage still contribute to cost. Multiple snakes in the
same game remain separate through their snake IDs; repeated API attempts count
separately because they can each incur charges.

As of 2026-09-17, [Jev 1.13.0 costs](https://docs.typesafe.ai/models)
$0.042 per million input tokens; output is free. Raw events retain
`estimated_cost_usd` and the applied rate. The dashboard uses **microUSD**
(1,000,000 = $1) so small amounts remain visible with Eyes' decimal precision.
Unknown resolved model versions have unknown cost until their pricing is verified
and added to `telemetry.rs`; changing the model alias cannot silently reuse an old rate.

These are telemetry estimates, not invoice totals: lost requests, process crashes,
or a full export queue can leave usage unavailable. Per-game tables include only
events in the selected time range, and Eyes' event retention applies. Allow a few
seconds for batching and late responses before comparing totals.

## Verify

```sh
cargo test -p web-axum
cargo test --workspace --locked
cargo clippy --workspace --lib --bins --tests --no-deps --locked -- -D warnings
curl -s http://localhost:3000/judicious-jev
curl -s -H 'Content-Type: application/json' \
  --data-binary @fixtures/jev-standard.json http://localhost:3000/judicious-jev/move
```

Tests use a local mock API and need no credentials. With the Battlesnake rules CLI:

```sh
battlesnake play --width 7 --height 7 --seed 1274 --timeout 500 \
  --name 'Judicious Jev' --url http://localhost:3000/judicious-jev \
  --name 'Bombastic Bob' --url http://localhost:3000/bombastic-bob
```

To inspect costs with the Eyes CLI (using a read-scoped token):

```sh
eyes named-metric jev.cost.by_game query --since 24h
eyes named-metric jev.input_tokens.by_game query --since 24h
eyes dashboards
```
