//! Behavioral-compatibility tests against the npm `serve` package.
//!
//! Expected values here were captured by running the real package locally
//! (`npx serve <fixture-dir>`) against the same fixture layout used below,
//! and comparing status codes / headers / bodies against this router.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use fast_static_server::build_app;
use tower::ServiceExt;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A throwaway site directory, unique per test, cleaned up on drop.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("fss_test_{}_{}", std::process::id(), id));
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(root.join("nolisting")).unwrap();
        fs::write(root.join("index.html"), "<h1>root index</h1>").unwrap();
        fs::write(root.join("sub/file.txt"), "hello sub").unwrap();
        fs::write(root.join("nolisting/a.txt"), "a").unwrap();
        fs::write(root.join("nolisting/b.txt"), "b").unwrap();
        Fixture { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

async fn body_string(res: Response) -> String {
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn serves_index_html_at_root() {
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_string(res).await, "<h1>root index</h1>");
}

#[tokio::test]
async fn serves_nested_static_file() {
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/sub/file.txt").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_string(res).await, "hello sub");
}

#[tokio::test]
async fn returns_404_for_missing_file_without_spa() {
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/does-not-exist").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn spa_mode_rewrites_unknown_routes_to_index() {
    // matches `serve -s`: unknown routes fall back to the root index.html instead of 404
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), true, false, false, None);
    let res = app
        .oneshot(
            Request::get("/some/client/route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_string(res).await, "<h1>root index</h1>");
}

#[tokio::test]
async fn lists_directory_alphabetically_when_no_index_html() {
    // real `serve` sorts directory entries by name only; it does not group directories first
    let fx = Fixture::new();
    fs::create_dir_all(fx.root.join("nolisting/zzz_dir")).unwrap();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/nolisting/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_string(res).await;
    let pos_a = body.find("a.txt").unwrap();
    let pos_b = body.find("b.txt").unwrap();
    let pos_z = body.find("zzz_dir").unwrap();
    assert!(
        pos_a < pos_b && pos_b < pos_z,
        "expected alphabetical order a.txt < b.txt < zzz_dir, got:\n{body}"
    );
}

#[tokio::test]
async fn directory_listing_links_work_without_trailing_slash() {
    // real `serve` serves the same listing for `/dir` and `/dir/`; the emitted links must
    // resolve to the same place in both cases (absolute paths, not relative to `/dir`'s parent)
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/nolisting").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_string(res).await;
    assert!(body.contains("href=\"/nolisting/a.txt\""), "body:\n{body}");
}

#[tokio::test]
async fn rejects_path_traversal() {
    let fx = Fixture::new();

    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/../outside").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::OK);

    let app2 = build_app(fx.root.clone(), false, false, false, None);
    let res2 = app2
        .oneshot(
            Request::get("/%2e%2e/%2e%2e/etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(res2.status(), StatusCode::OK);
}

#[tokio::test]
async fn range_requests_return_partial_content_like_serve() {
    // real `serve` (via the `send` library) supports byte-range requests -> 206 Partial Content
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(
            Request::get("/")
                .header(header::RANGE, "bytes=0-4")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(body_string(res).await, "<h1>r");
}

#[tokio::test]
async fn cors_header_absent_by_default_present_with_flag() {
    // real `serve` sends no CORS headers unless explicitly enabled
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(
            Request::get("/")
                .header(header::ORIGIN, "http://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        !res.headers()
            .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
    );

    let fx2 = Fixture::new();
    let app2 = build_app(fx2.root.clone(), false, true, false, None);
    let res2 = app2
        .oneshot(
            Request::get("/")
                .header(header::ORIGIN, "http://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res2.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "*"
    );
}

#[tokio::test]
async fn conditional_get_returns_304_without_any_flag() {
    // real `serve`'s caching is conditional-request based (ETag/Last-Modified -> 304);
    // this works out of the box via the underlying file service, no opt-in needed.
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let last_modified = res.headers().get(header::LAST_MODIFIED).unwrap().clone();

    let app2 = build_app(fx.root.clone(), false, false, false, None);
    let res2 = app2
        .oneshot(
            Request::get("/")
                .header(header::IF_MODIFIED_SINCE, last_modified)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn cache_control_absent_by_default_present_with_flag() {
    // real `serve` never sends Cache-Control; ours only does when `-c/--cache-control` is set
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, None);
    let res = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(!res.headers().contains_key(header::CACHE_CONTROL));

    let fx2 = Fixture::new();
    let app2 = build_app(fx2.root.clone(), false, false, false, Some(3600));
    let res2 = app2
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        res2.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=3600"
    );
}

#[tokio::test]
async fn cache_control_flag_does_not_leak_onto_directory_listing_or_404() {
    let fx = Fixture::new();
    let app = build_app(fx.root.clone(), false, false, false, Some(3600));
    let listing = app
        .oneshot(Request::get("/nolisting/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(!listing.headers().contains_key(header::CACHE_CONTROL));

    let app2 = build_app(fx.root.clone(), false, false, false, Some(3600));
    let not_found = app2
        .oneshot(Request::get("/does-not-exist").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(!not_found.headers().contains_key(header::CACHE_CONTROL));
}
