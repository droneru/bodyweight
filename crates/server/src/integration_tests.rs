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

impl TestApp {
    async fn exercises(&self, cookie: &str) -> Value {
        let (status, body) = self
            .send(Method::GET, "/api/exercises", Some(cookie), None)
            .await;
        assert_eq!(status, StatusCode::OK);
        serde_json::from_str(&body).unwrap()
    }

    /// Creates a custom exercise and returns its id.
    async fn create_exercise(&self, cookie: &str, name: &str) -> String {
        let body = Some(json!({ "name": name }));
        let (status, body) = self
            .send(Method::POST, "/api/exercises", Some(cookie), body)
            .await;
        assert_eq!(status, StatusCode::CREATED);
        let created: Value = serde_json::from_str(&body).unwrap();
        created["id"].as_str().unwrap().to_owned()
    }

    async fn patch_exercise(&self, cookie: &str, id: &str, body: Value) -> (StatusCode, String) {
        let path = format!("/api/exercises/{id}");
        self.send(Method::PATCH, &path, Some(cookie), Some(body))
            .await
    }

    async fn delete_exercise(&self, cookie: &str, id: &str) -> StatusCode {
        let path = format!("/api/exercises/{id}");
        self.send(Method::DELETE, &path, Some(cookie), None).await.0
    }
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn exercises_require_a_session() {
    let app = TestApp::new().await;

    let (status, _) = app.send(Method::GET, "/api/exercises", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let body = Some(json!({ "name": "Dips" }));
    let (status, _) = app.send(Method::POST, "/api/exercises", None, body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn a_new_user_has_only_the_standard_exercises() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;

    assert_eq!(
        app.exercises(&cookie).await,
        json!({
            "standard": [
                { "id": "pull-ups", "name": "Pull-ups" },
                { "id": "bench-press", "name": "Bench press" },
                { "id": "squat", "name": "Squat" },
                { "id": "deadlift", "name": "Deadlift" },
            ],
            "custom": [],
        })
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn custom_exercises_are_created_renamed_archived_and_deleted() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;

    let press = app.create_exercise(&cookie, "  overhead press ").await;
    let dips = app.create_exercise(&cookie, "Dips").await;
    assert_eq!(
        app.exercises(&cookie).await["custom"],
        json!([
            { "id": dips, "name": "Dips", "archived": false },
            { "id": press, "name": "overhead press", "archived": false },
        ])
    );

    // Renaming keeps the archived flag, archiving keeps the name.
    let (status, body) = app
        .patch_exercise(&cookie, &press, json!({ "name": "Barbell press" }))
        .await;
    assert_eq!((status, body.as_str()), (StatusCode::NO_CONTENT, ""));
    let (status, _) = app
        .patch_exercise(&cookie, &dips, json!({ "archived": true }))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.exercises(&cookie).await["custom"],
        json!([
            { "id": press, "name": "Barbell press", "archived": false },
            { "id": dips, "name": "Dips", "archived": true },
        ])
    );

    let (status, _) = app
        .patch_exercise(&cookie, &dips, json!({ "archived": false }))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(app.exercises(&cookie).await["custom"][1]["archived"], false);

    assert_eq!(
        app.delete_exercise(&cookie, &dips).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.delete_exercise(&cookie, &dips).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.exercises(&cookie).await["custom"],
        json!([{ "id": press, "name": "Barbell press", "archived": false }])
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn custom_exercises_are_private_to_the_user() {
    let app = TestApp::new().await;
    let (alice, bob) = (app.sign_in().await, app.sign_in().await);
    let dips = app.create_exercise(&alice, "Dips").await;

    assert_eq!(app.exercises(&bob).await["custom"], json!([]));
    let (status, _) = app
        .patch_exercise(&bob, &dips, json!({ "name": "Stolen" }))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        app.delete_exercise(&bob, &dips).await,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        app.exercises(&alice).await["custom"],
        json!([{ "id": dips, "name": "Dips", "archived": false }])
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn invalid_custom_exercises_are_rejected() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;
    let dips = app.create_exercise(&cookie, "Dips").await;

    let too_long = "x".repeat(61);
    let cases = [
        ("  ", "exercise name must not be empty"),
        (
            too_long.as_str(),
            "exercise name must be at most 60 characters",
        ),
        (
            "Dips\nweighted",
            "exercise name must be a single line of text",
        ),
    ];
    for (name, message) in cases {
        let body = Some(json!({ "name": name }));
        let (status, body) = app
            .send(Method::POST, "/api/exercises", Some(&cookie), body)
            .await;
        assert_eq!((status, body.as_str()), (StatusCode::BAD_REQUEST, message));

        let (status, body) = app
            .patch_exercise(&cookie, &dips, json!({ "name": name }))
            .await;
        assert_eq!((status, body.as_str()), (StatusCode::BAD_REQUEST, message));
    }

    // Standard exercises cannot be changed or deleted.
    let (status, _) = app
        .patch_exercise(&cookie, "squat", json!({ "name": "Squats" }))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        app.delete_exercise(&cookie, "squat").await,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        app.exercises(&cookie).await["custom"],
        json!([{ "id": dips, "name": "Dips", "archived": false }])
    );
}

const WORKOUT_A: &str = "000000000000000000000000000000aa";
const WORKOUT_B: &str = "000000000000000000000000000000bb";
const WORKOUT_C: &str = "000000000000000000000000000000cc";

impl TestApp {
    async fn put_workout(&self, cookie: &str, id: &str, body: Value) -> (StatusCode, String) {
        let path = format!("/api/workouts/{id}");
        self.send(Method::PUT, &path, Some(cookie), Some(body))
            .await
    }

    /// The workout, or `None` if there is no such workout.
    async fn workout(&self, cookie: &str, id: &str) -> Option<Value> {
        let path = format!("/api/workouts/{id}");
        let (status, body) = self.send(Method::GET, &path, Some(cookie), None).await;
        if status == StatusCode::NOT_FOUND {
            return None;
        }
        assert_eq!(status, StatusCode::OK);
        Some(serde_json::from_str(&body).unwrap())
    }

    async fn workouts(&self, cookie: &str) -> Value {
        let (status, body) = self
            .send(Method::GET, "/api/workouts", Some(cookie), None)
            .await;
        assert_eq!(status, StatusCode::OK);
        serde_json::from_str(&body).unwrap()
    }

    async fn delete_workout(&self, cookie: &str, id: &str) -> StatusCode {
        let path = format!("/api/workouts/{id}");
        self.send(Method::DELETE, &path, Some(cookie), None).await.0
    }
}

fn empty_workout(date: &str) -> Value {
    json!({ "date": date, "note": "", "entries": [] })
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn workouts_require_a_session() {
    let app = TestApp::new().await;

    let (status, _) = app.send(Method::GET, "/api/workouts", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let path = format!("/api/workouts/{WORKOUT_A}");
    let body = Some(empty_workout("2026-11-01"));
    let (status, _) = app.send(Method::PUT, &path, None, body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn a_workout_is_saved_with_its_entries_and_sets_in_order() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;
    let dips = app.create_exercise(&cookie, "Dips").await;
    assert_eq!(app.workout(&cookie, WORKOUT_A).await, None);

    // The example of docs/spec.md §1.3, plus a repeated exercise, a set without a weight
    // and an entry without sets.
    let body = json!({
        "date": "2026-11-01",
        "note": "  felt strong \n",
        "entries": [
            { "exercise_id": "bench-press", "sets": [{ "reps": 3, "weight": "72.50" }] },
            { "exercise_id": "squat", "sets": [
                { "reps": 5, "weight": "60" },
                { "reps": 5, "weight": "70" },
                { "reps": 5, "weight": "70" },
                { "reps": 5, "weight": "70" },
            ] },
            { "exercise_id": dips, "sets": [
                { "reps": 12, "weight": null },
                { "reps": 8, "weight": "11.25" },
            ] },
            { "exercise_id": "pull-ups", "sets": [] },
            { "exercise_id": "bench-press", "sets": [{ "reps": 10, "weight": "50" }] },
        ],
    });
    let (status, text) = app.put_workout(&cookie, WORKOUT_A, body).await;
    assert_eq!((status, text.as_str()), (StatusCode::NO_CONTENT, ""));

    assert_eq!(
        app.workout(&cookie, WORKOUT_A).await,
        Some(json!({
            "date": "2026-11-01",
            "note": "felt strong",
            "entries": [
                { "exercise_id": "bench-press", "sets": [{ "reps": 3, "weight": "72.5" }] },
                { "exercise_id": "squat", "sets": [
                    { "reps": 5, "weight": "60" },
                    { "reps": 5, "weight": "70" },
                    { "reps": 5, "weight": "70" },
                    { "reps": 5, "weight": "70" },
                ] },
                { "exercise_id": dips, "sets": [
                    { "reps": 12, "weight": null },
                    { "reps": 8, "weight": "11.25" },
                ] },
                { "exercise_id": "pull-ups", "sets": [] },
                { "exercise_id": "bench-press", "sets": [{ "reps": 10, "weight": "50" }] },
            ],
        }))
    );
    assert_eq!(
        app.workouts(&cookie).await,
        json!([{
            "id": WORKOUT_A,
            "date": "2026-11-01",
            "note": "felt strong",
            "exercise_ids": ["bench-press", "squat", dips, "pull-ups"],
        }])
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn saving_a_workout_again_replaces_it_as_a_whole() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;

    let first = json!({
        "date": "2026-11-01",
        "note": "legs",
        "entries": [
            { "exercise_id": "squat", "sets": [
                { "reps": 5, "weight": "60" },
                { "reps": 5, "weight": "70" },
            ] },
            { "exercise_id": "deadlift", "sets": [{ "reps": 3, "weight": "100" }] },
        ],
    });
    app.put_workout(&cookie, WORKOUT_A, first).await;

    let second = json!({
        "date": "2026-11-02",
        "note": "",
        "entries": [{ "exercise_id": "pull-ups", "sets": [{ "reps": 9, "weight": null }] }],
    });
    let (status, _) = app.put_workout(&cookie, WORKOUT_A, second.clone()).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(app.workout(&cookie, WORKOUT_A).await, Some(second));

    let (status, _) = app
        .put_workout(&cookie, WORKOUT_A, empty_workout("2026-11-02"))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.workout(&cookie, WORKOUT_A).await,
        Some(empty_workout("2026-11-02"))
    );
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn workouts_are_listed_newest_first() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;
    assert_eq!(app.workouts(&cookie).await, json!([]));

    // Two workouts on the same date: the one created later comes first.
    for (id, date) in [
        (WORKOUT_B, "2026-11-03"),
        (WORKOUT_C, "2026-11-01"),
        (WORKOUT_A, "2026-11-03"),
    ] {
        let (status, _) = app.put_workout(&cookie, id, empty_workout(date)).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    let ids = |list: Value| -> Vec<String> {
        let list = list.as_array().unwrap().iter();
        list.map(|w| w["id"].as_str().unwrap().to_owned()).collect()
    };
    assert_eq!(
        ids(app.workouts(&cookie).await),
        [WORKOUT_A, WORKOUT_B, WORKOUT_C]
    );

    // Saving a workout again does not move it within its date.
    app.put_workout(&cookie, WORKOUT_B, empty_workout("2026-11-03"))
        .await;
    assert_eq!(
        ids(app.workouts(&cookie).await),
        [WORKOUT_A, WORKOUT_B, WORKOUT_C]
    );

    assert_eq!(
        app.delete_workout(&cookie, WORKOUT_A).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.delete_workout(&cookie, WORKOUT_A).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.workout(&cookie, WORKOUT_A).await, None);
    assert_eq!(ids(app.workouts(&cookie).await), [WORKOUT_B, WORKOUT_C]);
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn workouts_are_private_to_the_user() {
    let app = TestApp::new().await;
    let (alice, bob) = (app.sign_in().await, app.sign_in().await);
    let dips = app.create_exercise(&alice, "Dips").await;

    let squats = json!({
        "date": "2026-11-01",
        "note": "alice",
        "entries": [{ "exercise_id": "squat", "sets": [{ "reps": 5, "weight": "60" }] }],
    });
    app.put_workout(&alice, WORKOUT_A, squats.clone()).await;

    assert_eq!(app.workouts(&bob).await, json!([]));
    assert_eq!(app.workout(&bob, WORKOUT_A).await, None);
    assert_eq!(
        app.delete_workout(&bob, WORKOUT_A).await,
        StatusCode::NOT_FOUND
    );

    // The same id is another workout for another user, and custom exercises are not shared.
    let (status, _) = app
        .put_workout(&bob, WORKOUT_A, empty_workout("2026-11-02"))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let with_dips = json!({
        "date": "2026-11-02",
        "note": "",
        "entries": [{ "exercise_id": dips, "sets": [] }],
    });
    let (status, text) = app.put_workout(&bob, WORKOUT_B, with_dips).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(text, format!("unknown exercise {dips:?}"));

    assert_eq!(app.workout(&alice, WORKOUT_A).await, Some(squats));
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn invalid_workouts_are_rejected() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;

    let with_set = |set: Value| {
        json!({
            "date": "2026-11-01",
            "note": "",
            "entries": [{ "exercise_id": "squat", "sets": [set] }],
        })
    };
    let many_sets = vec![json!({ "reps": 1, "weight": null }); 51];
    let many_entries = vec![json!({ "exercise_id": "squat", "sets": [] }); 51];
    let cases = [
        (
            empty_workout("01.11.2026"),
            "date must be in the YYYY-MM-DD format",
        ),
        (
            empty_workout("2999-01-01"),
            "date must be at most 2 days ahead of today",
        ),
        (empty_workout("1969-12-31"), "date is too far in the past"),
        (
            with_set(json!({ "reps": 0, "weight": null })),
            "reps must be between 1 and 1000",
        ),
        (
            with_set(json!({ "reps": 5, "weight": "70.1" })),
            "weight step is 0.25 kg",
        ),
        (
            with_set(json!({ "reps": 5, "weight": "0" })),
            "weight must be greater than 0 and at most 1000 kg",
        ),
        (
            with_set(json!({ "reps": 5, "weight": "70,5" })),
            "weight must be a number of kilograms, e.g. 72.5",
        ),
        (
            json!({ "date": "2026-11-01", "note": "x".repeat(2001), "entries": [] }),
            "note must be at most 2000 characters",
        ),
        (
            json!({ "date": "2026-11-01", "note": "", "entries": many_entries }),
            "a workout can have at most 50 exercises",
        ),
        (
            json!({
                "date": "2026-11-01",
                "note": "",
                "entries": [{ "exercise_id": "squat", "sets": many_sets }],
            }),
            "an exercise can have at most 50 sets",
        ),
        (
            json!({
                "date": "2026-11-01",
                "note": "",
                "entries": [{ "exercise_id": "lunges", "sets": [] }],
            }),
            "unknown exercise \"lunges\"",
        ),
    ];
    for (body, message) in cases {
        let (status, text) = app.put_workout(&cookie, WORKOUT_A, body).await;
        assert_eq!((status, text.as_str()), (StatusCode::BAD_REQUEST, message));
    }

    let (status, text) = app
        .put_workout(&cookie, "not-an-id", empty_workout("2026-11-01"))
        .await;
    assert_eq!(
        (status, text.as_str()),
        (
            StatusCode::BAD_REQUEST,
            "workout id must be 32 lowercase hex digits"
        )
    );
    assert_eq!(app.workout(&cookie, "not-an-id").await, None);
    assert_eq!(app.workouts(&cookie).await, json!([]));
}

#[tokio::test]
#[ignore = "needs YDB, run `make integration-test`"]
async fn a_custom_exercise_used_in_a_workout_cannot_be_deleted() {
    let app = TestApp::new().await;
    let cookie = app.sign_in().await;
    let dips = app.create_exercise(&cookie, "Dips").await;

    let with_dips = json!({
        "date": "2026-11-01",
        "note": "",
        "entries": [{ "exercise_id": dips, "sets": [{ "reps": 10, "weight": null }] }],
    });
    app.put_workout(&cookie, WORKOUT_A, with_dips.clone()).await;

    let path = format!("/api/exercises/{dips}");
    let (status, text) = app.send(Method::DELETE, &path, Some(&cookie), None).await;
    assert_eq!(
        (status, text.as_str()),
        (
            StatusCode::CONFLICT,
            "exercise is used in a workout, archive it instead"
        )
    );

    // It can be archived, and stays in the workout.
    let (status, _) = app
        .patch_exercise(&cookie, &dips, json!({ "archived": true }))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(app.workout(&cookie, WORKOUT_A).await, Some(with_dips));

    // Once no workout uses it, it can be deleted.
    app.put_workout(&cookie, WORKOUT_A, empty_workout("2026-11-01"))
        .await;
    assert_eq!(
        app.delete_exercise(&cookie, &dips).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(app.exercises(&cookie).await["custom"], json!([]));
}
