use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use meetagain::{
    api::{ApiState, router},
    graph::RoadGraph,
    simulation::{SimulationConfig, simulate},
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;

fn app() -> Router {
    router(
        Arc::new(ApiState::new(RoadGraph::bundled().unwrap())),
        "/tmp/meetagain-no-assets",
    )
}
fn config() -> Value {
    json!({"agents":20,"ticks":20,"seed":"20261116","movement":{"home_move_probability":0.1,"away_move_probability":0.25,"home_return_probability":0.6},"encounters":{"include_tick_zero":false,"cross_tick_tolerance":0}})
}
async fn send(app: Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let response = app
        .oneshot(
            builder
                .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert!(
        response.headers().contains_key("content-type"),
        "{method} {path}: status {status}"
    );
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, value)
}

#[tokio::test]
async fn graph_and_health_expose_consistent_model_and_normalized_boundaries() {
    let app = app();
    let (status, g) = send(app.clone(), "GET", "/api/graph", None).await;
    assert_eq!(status, 200);
    assert_eq!(g["edges"].as_array().unwrap().len(), 3);
    let ids: Vec<_> = g["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["320100", "321000", "321100"]);
    for f in g["boundaries"]["features"].as_array().unwrap() {
        assert!(ids.contains(&f["properties"]["city_id"].as_str().unwrap()));
    }
    let (status, health) = send(app, "GET", "/api/health", None).await;
    assert_eq!(status, 200);
    assert_eq!(health["status"], "ok");
    assert_eq!(health["graph_version"], g["graph_version"]);
}

#[tokio::test]
async fn street_api_serves_geojson_for_active_cities_only() {
    let app = app();
    let graph = RoadGraph::bundled().unwrap();
    for city in &graph.cities {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/map/roads/{}", city.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "application/geo+json");
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(Some(bytes), graph.streets(&city.id));
    }
    let (status, _) = send(app, "GET", "/api/map/roads/320400", None).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn api_matches_engine_and_pagination_has_no_missing_or_duplicate_events() {
    let app = app();
    let input = config();
    let engine = simulate(
        &RoadGraph::bundled().unwrap(),
        &serde_json::from_value::<SimulationConfig>(input.clone()).unwrap(),
    )
    .unwrap();
    let (status, result) = send(app.clone(), "POST", "/api/simulations", Some(input.clone())).await;
    assert_eq!(status, 201);
    assert_eq!(result["config"], input);
    assert_eq!(
        result["summary"],
        serde_json::to_value(&engine.summary).unwrap()
    );
    assert_eq!(
        result["agents"],
        serde_json::to_value(&engine.agents).unwrap()
    );
    assert!(result.get("encounters").is_none());
    let id = result["id"].as_str().unwrap();
    let mut path = format!("/api/simulations/{id}/encounters?limit=7");
    let mut events = Vec::new();
    loop {
        let (status, page) = send(app.clone(), "GET", &path, None).await;
        assert_eq!(status, 200);
        assert_eq!(page["simulation_id"], id);
        assert!(page["events"].as_array().unwrap().len() <= 7);
        events.extend(page["events"].as_array().unwrap().iter().cloned());
        match page["next_cursor"].as_str() {
            Some(cursor) => {
                path = format!("/api/simulations/{id}/encounters?limit=7&cursor={cursor}")
            }
            None => break,
        }
    }
    assert_eq!(
        Value::Array(events),
        serde_json::to_value(engine.encounters).unwrap()
    );
    for query in [
        "limit=0",
        "limit=1001",
        "limit=no",
        "cursor=other:1",
        "cursor=nonsense",
        "bogus=1",
    ] {
        assert_eq!(
            send(
                app.clone(),
                "GET",
                &format!("/api/simulations/{id}/encounters?{query}"),
                None
            )
            .await
            .0,
            422
        );
    }
}

#[tokio::test]
async fn errors_remain_json_with_correct_status_and_field() {
    let app = app();
    for path in [
        "/api",
        "/api/",
        "/api/missing",
        "/api/simulations/missing/encounters",
    ] {
        assert_eq!(send(app.clone(), "GET", path, None).await.0, 404);
    }
    assert_eq!(send(app.clone(), "POST", "/api/health", None).await.0, 405);
    let mut bad = config();
    bad["agents"] = 1.into();
    let (status, error) = send(app.clone(), "POST", "/api/simulations", Some(bad)).await;
    assert_eq!(status, 422);
    assert_eq!(error["error"]["field"], "agents");
    assert_eq!(
        send(app.clone(), "POST", "/api/simulations", Some(json!({})))
            .await
            .0,
        422
    );
    for (body, content_type, status) in [
        ("{", Some("application/json"), 400),
        ("{}", None, 415),
        (&" ".repeat(17000), Some("application/json"), 413),
    ] {
        let mut request = Request::post("/api/simulations");
        if let Some(t) = content_type {
            request = request.header("content-type", t);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert!(value.get("error").is_some());
    }
}

#[tokio::test]
async fn cache_evicts_oldest_run_and_enforces_expiration() {
    let app = app();
    let mut ids = Vec::new();
    for _ in 0..5 {
        let (_, r) = send(app.clone(), "POST", "/api/simulations", Some(config())).await;
        ids.push(r["id"].as_str().unwrap().to_owned());
    }
    assert_eq!(
        send(
            app.clone(),
            "GET",
            &format!("/api/simulations/{}/encounters", ids[0]),
            None
        )
        .await
        .0,
        404
    );
    assert_eq!(
        send(
            app,
            "GET",
            &format!("/api/simulations/{}/encounters", ids[4]),
            None
        )
        .await
        .0,
        200
    );
    let expired = router(
        Arc::new(ApiState::with_retention(
            RoadGraph::bundled().unwrap(),
            Duration::ZERO,
        )),
        "/tmp/meetagain-no-assets",
    );
    let (_, r) = send(expired.clone(), "POST", "/api/simulations", Some(config())).await;
    assert_eq!(
        send(
            expired,
            "GET",
            &format!("/api/simulations/{}/encounters", r["id"].as_str().unwrap()),
            None
        )
        .await
        .0,
        404
    );
}

#[tokio::test]
async fn serves_static_spa_without_api_fallback_leak() {
    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><h1>SPA</h1>").unwrap();
    let app = router(Arc::new(ApiState::new(RoadGraph::bundled().unwrap())), &dir);
    for path in ["/", "/results/123"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
    }
    assert_eq!(send(app, "GET", "/api/unknown", None).await.0, 404);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn invalid_utf8_id_keeps_the_api_error_envelope() {
    let (status, body) = send(app(), "GET", "/api/simulations/%FF/encounters", None).await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["field"], "id");
}
