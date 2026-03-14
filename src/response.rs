use axum::response::{Html, IntoResponse, Response};
use maud::Markup;

pub struct Template(pub Markup);

impl IntoResponse for Template {
    fn into_response(self) -> Response {
        Html(self.0.into_string()).into_response()
    }
}
