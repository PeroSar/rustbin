use std::sync::Arc;

use axum::{
    extract::{Multipart, Path as AxumPath, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use crate::{
    db::{insert_paste, load_paste_by_ref, load_paste_optional, sanitize_form},
    error::AppError,
    extractors::parse_create_paste_multipart,
    highlighter,
    preview,
    render::{index_page, paste_page, url_paste_page, usage_page},
    response::Template,
    state::{AppResult, AppState},
};

static FAVICON: &[u8] = include_bytes!("../assets/favicon.ico");
static LOGO: &[u8] = include_bytes!("../assets/logo.png");

pub async fn favicon() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/x-icon")], FAVICON)
}

pub async fn logo() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], LOGO)
}

pub async fn index() -> Template {
    Template(index_page(None))
}

pub async fn usage() -> Template {
    Template(usage_page())
}

pub async fn create_paste_multipart(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    multipart: Multipart,
) -> AppResult<Response> {
    let mut form = parse_create_paste_multipart(multipart).await?;
    let from_browser = form.from_browser;
    form.language = highlighter::detect_language(
        &state,
        form.filename.as_deref(),
        form.content.as_deref().unwrap_or_default(),
    );
    let content_is_url = form.content.as_deref().map_or(false, is_url);
    let destination_url = if content_is_url {
        form.content.as_deref().map(|c| c.trim().to_string())
    } else {
        None
    };
    let id = insert_paste(&state.db, sanitize_form(form)).await?;
    let location = build_paste_url(&headers, &id);

    if from_browser {
        if let Some(destination) = destination_url {
            return Ok(Template(url_paste_page(&location, &destination)).into_response());
        }
        return Ok((StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response());
    }

    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location.clone())],
        format!("{location}\n"),
    )
        .into_response())
}

pub async fn show_paste(
    State(state): State<Arc<AppState>>,
    AxumPath(paste_ref): AxumPath<String>,
) -> AppResult<Response> {
    if let Some(paste) = load_paste_by_ref(&state.db, &paste_ref).await? {
        if is_url(&paste.content) {
            let url = paste.content.trim().to_string();
            return Ok(
                (StatusCode::FOUND, [(header::LOCATION, url)]).into_response()
            );
        }

        let extension = paste_ref
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .or(paste.language.as_deref());
        let is_markdown = highlighter::is_markdown(extension);
        let render_cache_key = render_cache_key(&paste.id, extension);
        if let Some(content_html) = state.render_cache.lock().get(&render_cache_key).cloned() {
            return Ok(
                Template(paste_page(&paste_ref, &paste, &content_html, is_markdown)).into_response(),
            );
        }

        let content_html: Arc<str> = if is_markdown {
            highlighter::render_markdown(&state, &paste.content).into()
        } else {
            highlighter::render_content(&state, extension, &paste.content).into()
        };
        state
            .render_cache
            .lock()
            .put(render_cache_key, Arc::clone(&content_html));
        return Ok(
            Template(paste_page(&paste_ref, &paste, &content_html, is_markdown)).into_response(),
        );
    }

    Err(AppError::NotFound("Paste not found."))
}

fn is_url(content: &str) -> bool {
    let trimmed = content.trim();
    !trimmed.contains('\n')
        && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
}

pub async fn show_raw_paste(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> AppResult<Response> {
    let paste = load_paste_optional(&state.db, &id)
        .await?
        .ok_or(AppError::NotFound("Paste not found."))?;

    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        paste.content,
    )
        .into_response())
}

pub async fn show_preview(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> AppResult<Response> {
    let paste = load_paste_by_ref(&state.db, &id)
        .await?
        .ok_or(AppError::NotFound("Paste not found."))?;

    let extension = id
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .or(paste.language.as_deref());

    let cache_key = render_cache_key(&paste.id, extension);
    if let Some(cached) = state.preview_cache.lock().get(&cache_key).cloned() {
        return Ok((
            [(header::CONTENT_TYPE, "image/png")],
            cached.to_vec(),
        )
            .into_response());
    }

    let png_data = preview::generate_preview(&state, &paste.content, extension);
    let cached: Arc<[u8]> = png_data.into();
    state
        .preview_cache
        .lock()
        .put(cache_key, Arc::clone(&cached));

    Ok((
        [(header::CONTENT_TYPE, "image/png")],
        cached.to_vec(),
    )
        .into_response())
}

fn build_paste_url(headers: &HeaderMap, id: &str) -> String {
    let host = forwarded_header(headers, "x-forwarded-host")
        .or_else(|| header_value(headers, header::HOST.as_str()))
        .unwrap_or("localhost");
    let proto = forwarded_header(headers, "x-forwarded-proto").unwrap_or("http");
    let prefix = forwarded_header(headers, "x-forwarded-prefix").unwrap_or("");

    format!("{proto}://{host}{prefix}/{id}")
}

fn forwarded_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    header_value(headers, name)
        .and_then(|value| value.split(',').next())
        .map(str::trim)
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn render_cache_key(id: &str, extension: Option<&str>) -> String {
    match extension.map(str::trim).filter(|value| !value.is_empty()) {
        Some(extension) => format!("{id}:{extension}"),
        None => id.to_string(),
    }
}
