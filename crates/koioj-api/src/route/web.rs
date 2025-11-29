use crate::AppState;
use axum::{
    Router,
    http::{StatusCode, Uri},
    response::Response,
};
use std::sync::Arc;

pub fn top_routes() -> Router<Arc<AppState>> {
    Router::new().fallback(serve_frontend)
}

async fn serve_frontend(uri: Uri) -> Result<Response, StatusCode> {
    let path = uri.path().to_string();

    let (is_index, content) = match koioj_web::get_file(path.as_str()) {
        Some(content) => (path.as_str() == "/index.html", content),
        _ => (true, koioj_web::get_file("/index.html").unwrap()),
    };

    if is_index {
        let html = String::from_utf8_lossy(content);
        let replaced = html.replace(
            r#"window.API_ROOT = "{{ api_root }}";"#,
            r#"window.API_ROOT = window.location.origin;"#,
        );

        return Ok(Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(replaced.into())
            .unwrap());
    }

    let mime = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    return Ok(Response::builder()
        .header("Content-Type", mime)
        .body(content.to_vec().into())
        .unwrap());
}
