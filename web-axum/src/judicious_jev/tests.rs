use super::*;
use axum::http::HeaderMap;
use tokio::{sync::mpsc, task::JoinHandle};

fn game() -> Game {
    serde_json::from_str(include_str!("../../../fixtures/jev-standard.json")).unwrap()
}

struct Server {
    url: String,
    task: JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn serve(app: Router) -> Server {
    logs();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = axum::Server::from_tcp(listener)
        .unwrap()
        .serve(app.into_make_service());
    let task = tokio::spawn(async move {
        server.await.unwrap();
    });
    Server { url, task }
}

fn answer(direction: &str) -> Value {
    json!({"model": "jev-1.13.0", "usage": {"input_tokens": 1200, "output_tokens": 25},
        "answers": {"move": {"type": "choice", "choice": direction,
        "confidence": 0.9, "probabilities": {direction: 1.0}}}})
}

fn mock(status: StatusCode, response: Value, delay: Duration) -> (Server, mpsc::Receiver<Value>) {
    let (tx, rx) = mpsc::channel(8);
    let server = serve(Router::new().route(
        "/",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let tx = tx.clone();
            let response = response.clone();
            async move {
                assert_eq!(headers["authorization"], "Bearer test-key");
                tx.send(body).await.unwrap();
                tokio::time::sleep(delay).await;
                (status, Json(response))
            }
        }),
    ));
    (server, rx)
}

fn jev(server: &Server) -> Arc<Jev> {
    Arc::new(Jev::new(Some("test-key"), &server.url, "jev-latest".into()).unwrap())
}

#[tokio::test]
async fn sends_choice_contract_and_uses_selected_move() {
    let (server, mut requests) = mock(StatusCode::OK, answer("left"), Duration::ZERO);
    let output = jev(&server).make_move(&game()).await;
    assert_eq!(output.r#move, "left");
    assert_eq!(output.shout.as_deref(), Some("A considered choice."));
    let body = requests.recv().await.unwrap();
    assert_eq!(body["model"], "jev-latest");
    assert_eq!(body["questions"]["move"]["type"], "choice");
    assert!(body["questions"]["move"]["criteria"].get("down").is_none());
    assert!(body["state"]["board"]["snakes"][0].get("name").is_none());
}

#[tokio::test]
async fn errors_and_invalid_choices_use_local_fallback() {
    let g = game();
    let fallback = board::fallback(&g, &board::candidates(&g));
    for (status, response) in [
        (StatusCode::UNAUTHORIZED, json!({"error": "unauthorized"})),
        (StatusCode::TOO_MANY_REQUESTS, json!({"error": "busy"})),
        (
            StatusCode::from_u16(529).unwrap(),
            json!({"error": "overloaded"}),
        ),
        (StatusCode::OK, answer("down")),
        (StatusCode::OK, answer("diagonal")),
        (StatusCode::OK, json!({"answers": {}})),
    ] {
        let (server, _requests) = mock(status, response, Duration::ZERO);
        let output = jev(&server).make_move(&g).await;
        assert_eq!(output.r#move, fallback);
        assert_eq!(output.shout.as_deref(), Some("Going with my gut."));
    }
}

#[tokio::test]
async fn deadline_returns_fallback_while_late_usage_request_finishes() {
    let (server, mut requests) = mock(StatusCode::OK, answer("left"), Duration::from_millis(200));
    let jev = jev(&server);
    let mut g = game();
    g.game.timeout = 100;
    let started = Instant::now();
    let output = jev.make_move(&g).await;
    assert!(started.elapsed() < Duration::from_millis(150));
    assert_eq!(output.shout.as_deref(), Some("Going with my gut."));
    requests.recv().await.unwrap();
    assert_eq!(jev.in_flight.available_permits(), 31);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(jev.in_flight.available_permits(), 32);
}

#[tokio::test]
async fn missing_key_no_time_and_capacity_do_not_call_api() {
    let (server, mut requests) = mock(StatusCode::OK, answer("left"), Duration::ZERO);
    let offline = Arc::new(Jev::new(None, &server.url, "jev-latest".into()).unwrap());
    assert_eq!(
        offline.make_move(&game()).await.shout.as_deref(),
        Some("Going with my gut.")
    );
    let jev = jev(&server);
    let mut g = game();
    g.game.timeout = 30;
    jev.make_move(&g).await;
    let _permits = jev.in_flight.clone().acquire_many_owned(32).await.unwrap();
    jev.make_move(&game()).await;
    assert!(requests.try_recv().is_err());
}

#[tokio::test]
async fn battlesnake_routes_work_alongside_dynamic_routes() {
    let (api, _requests) = mock(StatusCode::OK, answer("left"), Duration::ZERO);
    let server = serve(
        Router::new()
            .route("/:snake_name", get(|| async { "legacy" }))
            .merge(routes(jev(&api))),
    );
    let client = Client::new();
    let info: Value = client
        .get(format!("{}/judicious-jev", server.url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(info["apiversion"], "1");
    for route in ["start", "end"] {
        assert_eq!(
            client
                .post(format!("{}/judicious-jev/{route}", server.url))
                .json(&game())
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
    }
    let movement: Value = client
        .post(format!("{}/judicious-jev/move", server.url))
        .json(&game())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(movement["move"], "left");
    assert_eq!(
        client
            .get(format!("{}/other", server.url))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "legacy"
    );
}

#[derive(Clone)]
struct LogWriter(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn logs() -> &'static LogWriter {
    static LOGS: std::sync::OnceLock<LogWriter> = std::sync::OnceLock::new();
    LOGS.get_or_init(|| {
        let writer = LogWriter(Arc::new(std::sync::Mutex::new(Vec::new())));
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_writer(writer.clone())
            .finish();
        // Initialize before any test server starts so parallel requests cannot
        // capture an absent subscriber or register disabled telemetry callsites.
        tracing::subscriber::set_global_default(subscriber).unwrap();
        writer
    })
}

#[tokio::test]
async fn late_and_invalid_answers_still_record_billable_usage() {
    let writer = logs();
    async {
        let mut invalid = answer("left");
        invalid["answers"] = json!({});
        let (server, _requests) = mock(StatusCode::OK, invalid, Duration::from_millis(80));
        let jev = jev(&server);
        let mut g = game();
        g.game.id = "jev-usage-regression".into();
        g.game.timeout = 60;
        assert_eq!(
            jev.make_move(&g).await.shout.as_deref(),
            Some("Going with my gut.")
        );
        let _finished = tokio::time::timeout(
            Duration::from_secs(2),
            jev.in_flight.clone().acquire_many_owned(32),
        )
        .await
        .unwrap()
        .unwrap();
    }
    .await;
    let log = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    let records: Vec<Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let usage = &records
        .iter()
        .find(|r| {
            r["fields"]["event_kind"] == "jev_api"
                && r["fields"]["game_id"] == "jev-usage-regression"
        })
        .unwrap_or_else(|| panic!("Missing usage event in {log}"))["fields"];
    assert_eq!(usage["input_tokens"], 1200);
    assert_eq!(usage["output_tokens"], 25);
    assert!((usage["estimated_cost_usd"].as_f64().unwrap() - 0.0000504).abs() < 1e-12);
    assert_eq!(usage["cost_unknown"], 0);
    assert_eq!(usage["game_id"], "jev-usage-regression");
    let movement = &records
        .iter()
        .find(|r| {
            r["fields"]["event_kind"] == "jev_move"
                && r["fields"]["game_id"] == "jev-usage-regression"
        })
        .unwrap()["fields"];
    assert_eq!(movement["reason"], "timeout");
    assert_eq!(movement["fallback"], 1);
}
