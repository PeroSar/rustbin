use std::{collections::HashMap, sync::Arc};

use lru::LruCache;
use parking_lot::Mutex;
use sqlx::SqlitePool;
use syntect::{highlighting::Theme, parsing::SyntaxSet};

use crate::error::AppError;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub syntax_set: Arc<SyntaxSet>,
    pub syntax_index_by_token: Arc<HashMap<String, usize>>,
    pub render_cache: Arc<Mutex<LruCache<String, Arc<str>>>>,
    pub theme: Arc<Theme>,
}

#[derive(Debug, Default)]
pub struct CreatePasteForm {
    pub expires_in: Option<String>,
    pub filename: Option<String>,
    pub language: Option<String>,
    pub content: Option<String>,
    pub from_browser: bool,
}

#[derive(Debug)]
pub struct Paste {
    pub id: String,
    pub language: Option<String>,
    pub content: String,
}

pub type AppResult<T> = Result<T, AppError>;
