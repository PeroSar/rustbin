use axum::extract::Multipart;

use crate::{
    error::bad_request,
    state::{AppResult, CreatePasteForm},
};

pub async fn parse_create_paste_multipart(mut multipart: Multipart) -> AppResult<CreatePasteForm> {
    let mut form = CreatePasteForm::default();

    while let Some(field) = multipart.next_field().await.map_err(bad_request)? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                form.filename = field.file_name().map(str::to_string);
                let value = field.text().await.map_err(bad_request)?;
                form.content = Some(value);
            }
            "content" => {
                form.from_browser = true;
                let value = field.text().await.map_err(bad_request)?;
                form.content = Some(value);
            }
            "expires_in" => {
                let value = field.text().await.map_err(bad_request)?;
                form.expires_in = Some(value);
            }
            _ => {
                let _ = field.bytes().await.map_err(bad_request)?;
            }
        }
    }

    if form.content.is_none() {
        return Err(crate::error::AppError::BadRequest(
            "Missing multipart file field `file`.".into(),
        ));
    }

    Ok(form)
}
