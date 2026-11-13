//! Integration tests of the HTTP API. They need a YDB with the migrations applied
//! (`YDB_ENDPOINT`, `YDB_DATABASE`, credentials) and are run by `make integration-test`.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use bodyweight_storage::Storage;
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::{AppState, api};

struct TestApp {
    router: Router,
}

impl TestApp {
    async fn new() -> Self {
        let storage = Storage::connect_from_env()
            .await
            .expect("connect to the test YDB");
        let state = AppState {
            storage: Arc::new(storage),
            yandex: None,
            secure_cookies: false,
        };
        Self {
            router: Router::new().nest("/api", api(true).with_state(state)),
        }
    }

    /// Signs in a fresh user through the dev login and returns their `Cookie` header.
    async fn sign_in(&self) -> String {
        let login = format!("it-{}", hex::encode(rand::random::<[u8; 8]>()));
        let request = Request::post("/api/auth/dev-login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "login": login }).to_string()))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        set_cookie.split(';').next().unwrap().to_owned()
    }

    /// Sends a request and returns the status and the body as text.
    async fn send(
        &self,
        method: Method,
        path: &str,
        cookie: Option<&str>,
        json: Option<Value>,
    ) -> (StatusCode, String) {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let body = match json {
            Some(json) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .router
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    async fn put_weigh_in(
        &self,
        cookie: &str,
        date: &str,
        bodyweight: &str,
    ) -> (StatusCode, String) {
        let path = format!("/api/weigh-ins/{date}");
        let body = json!({ "bodyweight": bodyweight });
        self.send(Method::PUT, &path, Some(cookie), Some(body))
            .await
    }

    async fn weigh_ins(&self, cookie: &str) -> Value {
        let (status, body) = self
            .send(Method::GET, "/api/weigh-ins", Some(cookie), None)
            .await;
        assert_eq!(status, StatusCode::OK);
        serde_json::from_str(&body).unwrap()
    }
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn weigh_ins_require_a_session() {
    let app = TestApp::new().await;

    let (status, _) = app.send(Method::GET, "/api/weigh-ins", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let stale = Some("bodyweight_session=nope");
    let body = Some(json!({ "bodyweight": "82.5" }));
    let (status, _) = app
        .send(Method::PUT, "/api/weigh-ins/2026-11-01", stale, body)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn weigh_ins_are_added_listed_newest_first_and_replaced() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;
    assert_eq!(app.weigh_ins(&cookie).await, json!([]));

    for (date, bodyweight) in [("2026-11-01", "82"), ("2026-11-03", "81.5")] {
        let (status, body) = app.put_weigh_in(&cookie, date, bodyweight).await;
        assert_eq!((status, body.as_str()), (StatusCode::NO_CONTENT, ""));
    }
    assert_eq!(
        app.weigh_ins(&cookie).await,
        json!([
            { "date": "2026-11-03", "bodyweight": "81.5" },
            { "date": "2026-11-01", "bodyweight": "82.0" },
        ])
    );

    // A new weigh-in on the same date replaces the old one.
    let (status, _) = app.put_weigh_in(&cookie, "2026-11-01", "83.1").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.weigh_ins(&cookie).await,
        json!([
            { "date": "2026-11-03", "bodyweight": "81.5" },
            { "date": "2026-11-01", "bodyweight": "83.1" },
        ])
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn weigh_ins_are_private_to_the_user() {
    let app = TestApp::new().await;
    let (alice, bob) = (app.sign_in().await, app.sign_in().await);

    app.put_weigh_in(&alice, "2026-11-01", "60").await;
    app.put_weigh_in(&bob, "2026-11-01", "90").await;

    assert_eq!(
        app.weigh_ins(&alice).await,
        json!([{ "date": "2026-11-01", "bodyweight": "60.0" }])
    );
    assert_eq!(
        app.weigh_ins(&bob).await,
        json!([{ "date": "2026-11-01", "bodyweight": "90.0" }])
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn invalid_weigh_ins_are_rejected() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;

    let cases = [
        ("2026-11-01", "82.25", "bodyweight precision is 0.1 kg"),
        (
            "2026-11-01",
            "0",
            "bodyweight must be greater than 0 and at most 500 kg",
        ),
        (
            "2026-11-01",
            "82,5",
            "bodyweight must be a number of kilograms, e.g. 82.5",
        ),
        (
            "01.11.2026",
            "82.5",
            "date must be in the YYYY-MM-DD format",
        ),
        ("1969-12-31", "82.5", "date is too far in the past"),
        (
            "2999-01-01",
            "82.5",
            "date must be at most 2 days ahead of today",
        ),
    ];
    for (date, bodyweight, message) in cases {
        let (status, body) = app.put_weigh_in(&cookie, date, bodyweight).await;
        assert_eq!((status, body.as_str()), (StatusCode::BAD_REQUEST, message));
    }
    assert_eq!(app.weigh_ins(&cookie).await, json!([]));
}
