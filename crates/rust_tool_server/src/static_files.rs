use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
};
use std::path::{Component, PathBuf};

pub async fn serve_static(request: Request<Body>) -> Response<Body> {
    let path = sanitize_path(request.uri().path());
    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend/dist");
    let requested = dist_dir.join(path);
    let file_path = if requested.is_file() {
        requested
    } else {
        dist_dir.join("index.html")
    };

    match tokio::fs::read(&file_path).await {
        Ok(contents) => {
            let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(contents))
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("frontend dist not found"))
            .unwrap(),
    }
}

fn sanitize_path(path: &str) -> PathBuf {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return PathBuf::from("index.html");
    }

    let mut safe = PathBuf::new();
    for component in PathBuf::from(trimmed).components() {
        if let Component::Normal(part) = component {
            safe.push(part);
        }
    }

    if safe.as_os_str().is_empty() {
        PathBuf::from("index.html")
    } else {
        safe
    }
}
