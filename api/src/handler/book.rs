use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;
use uuid::Uuid;

use crate::model::book::{BookResponse, CreateBookRequest};
use registry::AppRegistry;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
    }
}

pub async fn register_book(
    State(register): State<AppRegistry>,
    Json(req): Json<CreateBookRequest>,
) -> Result<StatusCode, AppError> {
    register
        .book_repository()
        .create(req.into())
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(AppError::from)
}

pub async fn show_book_list(
    State(register): State<AppRegistry>,
) -> Result<Json<Vec<BookResponse>>, AppError> {
    register
        .book_repository()
        .find_all()
        .await
        .map(|v| v.into_iter().map(BookResponse::from).collect())
        .map(Json)
        .map_err(AppError::from)
}

pub async fn show_book(
    State(register): State<AppRegistry>,
    Path(id): Path<Uuid>,
) -> Result<Json<BookResponse>, AppError> {
    register
        .book_repository()
        .find_by_id(id)
        .await
        .and_then(|bc| match bc {
            Some(bc) => Ok(Json(bc.into())),
            None => Err(anyhow::anyhow!("The specified book does not found")),
        })
        .map_err(AppError::from)
}
