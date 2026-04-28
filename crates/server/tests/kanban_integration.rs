#[path = "kanban_integration/helpers.rs"]
mod helpers;

use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn smoke_test_list_projects() {
    let app = helpers::setup_test_app().await;

    let pool = helpers::setup_pool().await;
    let db = helpers::seed_db(pool.clone()).await;
    let (org_id, _, _) = helpers::get_seeded_ids(&db.pool).await;

    let response = app
        .oneshot(
            http::Request::builder()
                .uri(format!("/fallback/projects?organization_id={org_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(
        json.get("projects").is_some(),
        "response should contain 'projects' key"
    );
    let projects = json["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1, "should have exactly one seeded project");
    assert_eq!(projects[0]["name"], "Main Project");
}
