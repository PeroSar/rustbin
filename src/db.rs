use nanoid::nanoid;
use sqlx::{SqlitePool, migrate::Migrator};
use time::OffsetDateTime;
use tracing::debug;

use crate::{
    error::AppError,
    state::{AppResult, CreatePasteForm, Paste},
};

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn migrate_db(db: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(db).await
}

pub async fn insert_paste(db: &SqlitePool, form: CreatePasteForm) -> AppResult<String> {
    let content = form
        .content
        .ok_or(AppError::UnprocessableEntity("Can't paste empty input!"))?;

    if content.is_empty() {
        return Err(AppError::UnprocessableEntity("Can't paste empty input!"));
    }

    let expires_at = parse_expiry(form.expires_in.as_deref().unwrap_or("never"))
        .ok_or_else(|| AppError::BadRequest("Invalid expiry option.".into()))?;

    let id = nanoid!(10);
    let now = now_timestamp();
    let language = form.language;
    let content_len = content.len();
    let expires_in = form.expires_in;

    debug!(
        paste_id = %id,
        language = ?language,
        content_len,
        expires_in = ?expires_in,
        expires_at = ?expires_at,
        "inserting paste"
    );

    sqlx::query!(
        r#"
        INSERT INTO pastes (id, language, content, created_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        id,
        language,
        content,
        now,
        expires_at
    )
    .execute(db)
    .await
    .map_err(AppError::Internal)?;

    Ok(id)
}

pub async fn load_paste_by_ref(db: &SqlitePool, paste_ref: &str) -> AppResult<Option<Paste>> {
    if let Some(paste) = load_paste_optional(db, paste_ref).await? {
        return Ok(Some(paste));
    }

    if let Some((id, ext)) = paste_ref.rsplit_once('.')
        && !id.is_empty()
        && !ext.is_empty()
    {
        return Ok(load_paste_optional(db, id).await?);
    }

    Ok(None)
}

pub async fn load_paste_optional(db: &SqlitePool, id: &str) -> AppResult<Option<Paste>> {
    let now = now_timestamp();

    sqlx::query_as!(
        Paste,
        r#"
        SELECT
            id AS "id!: String",
            language AS "language?: String",
            content AS "content!: String"
        FROM pastes
        WHERE id = ?1
          AND (expires_at IS NULL OR expires_at > ?2)
        "#,
        id,
        now
    )
    .fetch_optional(db)
    .await
    .map_err(AppError::Internal)
}

pub fn sanitize_form(mut form: CreatePasteForm) -> CreatePasteForm {
    form.expires_in = Some(
        form.expires_in
            .unwrap_or_else(|| "never".into())
            .trim()
            .to_string(),
    );
    form.content = form
        .content
        .map(|value| value.trim_end_matches('\r').to_string());
    form.filename = form.filename.map(|value| value.trim().to_string());
    form.language = form.language.map(|value| value.trim().to_ascii_lowercase());
    debug!(
        filename = ?form.filename,
        language = ?form.language,
        expires_in = ?form.expires_in,
        content_len = form.content.as_deref().map(str::len).unwrap_or(0),
        "sanitized paste form"
    );
    form
}

fn parse_expiry(value: &str) -> Option<Option<i64>> {
    match value {
        "never" | "" => Some(None),
        seconds => seconds
            .parse::<i64>()
            .ok()
            .filter(|seconds| *seconds > 0)
            .map(|seconds| Some(now_timestamp() + seconds)),
    }
}

fn now_timestamp() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
