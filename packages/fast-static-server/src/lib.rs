use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use tower::ServiceExt;
use tower_http::{cors::CorsLayer, services::ServeFile};

/// Chars left unescaped in a percent-encoded path segment.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub struct AppState {
    pub root: PathBuf,
    pub spa: bool,
    /// `None` means no header, matching npm `serve`'s default.
    pub cache_control: Option<HeaderValue>,
}

/// `debug` gates per-request logging (off by default: npm `serve` is silent, and
/// logging unconditionally would throttle throughput under load).
pub fn build_app(
    root: PathBuf,
    spa: bool,
    cors: bool,
    debug: bool,
    cache_seconds: Option<u64>,
) -> Router {
    let cache_control = cache_seconds.map(|secs| {
        HeaderValue::from_str(&format!("public, max-age={secs}")).expect("valid header value")
    });
    let state = Arc::new(AppState {
        root,
        spa,
        cache_control,
    });

    let mut app = Router::new().fallback(handler).with_state(state);

    if debug {
        app = app.layer(middleware::from_fn(log_middleware));
    }
    if cors {
        app = app.layer(CorsLayer::permissive());
    }

    app
}

async fn log_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    let res = next.run(req).await;
    println!(
        "{} {} {} {:?}",
        method,
        res.status().as_u16(),
        path,
        start.elapsed()
    );
    res
}

/// Resolves a URL path against `root`, rejecting `..` traversal.
fn sanitize_path(root: &Path, url_path: &str) -> Option<PathBuf> {
    let decoded = percent_decode_str(url_path).decode_utf8().ok()?;
    let mut path = root.to_path_buf();
    for seg in decoded.split('/') {
        match seg {
            "" | "." => continue,
            ".." => return None,
            s => path.push(s),
        }
    }
    Some(path)
}

async fn handler(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let url_path = req.uri().path().to_string();
    let Some(fs_path) = sanitize_path(&state.root, &url_path) else {
        return (StatusCode::FORBIDDEN, "403 Forbidden").into_response();
    };

    // Stat first: opening a directory with `ServeFile` succeeds on Unix (unlike
    // Windows) and only errors once the body is read, so a blind open would return
    // a broken response instead of a clean 404.
    match tokio::fs::metadata(&fs_path).await {
        Ok(meta) if meta.is_file() => serve_file(&fs_path, req, state.cache_control.as_ref()).await,
        Ok(meta) if meta.is_dir() => {
            let index = fs_path.join("index.html");
            let res = serve_file(&index, req, state.cache_control.as_ref()).await;
            if res.status() != StatusCode::NOT_FOUND {
                return res;
            }
            directory_listing(&fs_path, &url_path).await
        }
        _ => {
            if state.spa {
                let root_index = state.root.join("index.html");
                if is_file(&root_index).await {
                    return serve_file(&root_index, req, state.cache_control.as_ref()).await;
                }
            }
            (StatusCode::NOT_FOUND, "404 Not Found").into_response()
        }
    }
}

async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.is_file())
        .unwrap_or(false)
}

async fn serve_file(path: &Path, req: Request, cache_control: Option<&HeaderValue>) -> Response {
    let res = ServeFile::new(path).oneshot(req).await.unwrap();
    let mut res = res.map(Body::new);
    if let Some(value) = cache_control {
        res.headers_mut()
            .insert(header::CACHE_CONTROL, value.clone());
    }
    res
}

async fn directory_listing(dir: &Path, url_path: &str) -> Response {
    let mut read_dir = match tokio::fs::read_dir(dir).await {
        Ok(rd) => rd,
        Err(_) => return (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    };

    let mut entries = Vec::new();
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
        if let Some(name) = entry.file_name().to_str() {
            entries.push((name.to_string(), is_dir));
        }
    }
    // npm `serve` sorts by name alone, no directories-first grouping.
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let base = if url_path.ends_with('/') {
        url_path.to_string()
    } else {
        format!("{url_path}/").to_string()
    };

    let title = escape_html(url_path);
    let mut html = format!(
        "<!doctype html><meta charset=utf-8><title>Index of {title}</title>\n<h1>Index of {title}</h1>\n<ul>\n"
    );
    if base != "/" {
        let trimmed = base.trim_end_matches('/');
        let parent = match trimmed.rfind('/') {
            Some(i) => &trimmed[..=i],
            None => "/",
        };
        html.push_str(&format!("<li><a href=\"{parent}\">../</a></li>\n"));
    }
    for (name, is_dir) in entries {
        let encoded = utf8_percent_encode(&name, PATH_SEGMENT).to_string();
        let display = escape_html(&name);
        if is_dir {
            html.push_str(&format!(
                "<li><a href=\"{base}{encoded}/\">{display}/</a></li>\n"
            ));
        } else {
            html.push_str(&format!(
                "<li><a href=\"{base}{encoded}\">{display}</a></li>\n"
            ));
        }
    }
    html.push_str("</ul>\n");

    Html(html).into_response()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_traversal() {
        let root = Path::new("/srv/www");
        assert_eq!(sanitize_path(root, "/../../etc/passwd"), None);
        assert_eq!(sanitize_path(root, "/a/../../b"), None);
        assert_eq!(
            sanitize_path(root, "/a/b.txt"),
            Some(root.join("a").join("b.txt"))
        );
        assert_eq!(sanitize_path(root, "/"), Some(root.to_path_buf()));
    }

    #[test]
    fn escape_html_neutralizes_markup() {
        assert_eq!(
            escape_html("<script>&\"</script>"),
            "&lt;script&gt;&amp;&quot;&lt;/script&gt;"
        );
    }
}
