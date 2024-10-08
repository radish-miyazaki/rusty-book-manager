use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use kernel::model::id::BookId;

use crate::model::book::{BookResponse, CreateBookRequest};
use registry::AppRegistry;
use shared::error::{AppError, AppResult};

pub async fn register_book(
    State(register): State<AppRegistry>,
    Json(req): Json<CreateBookRequest>,
) -> AppResult<StatusCode> {
    register
        .book_repository()
        .create(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

pub async fn show_book_list(
    State(register): State<AppRegistry>,
) -> AppResult<Json<Vec<BookResponse>>> {
    register
        .book_repository()
        .find_all()
        .await
        .map(|v| v.into_iter().map(BookResponse::from).collect())
        .map(Json)
}

pub async fn show_book(
    State(register): State<AppRegistry>,
    Path(book_id): Path<BookId>,
) -> AppResult<Json<BookResponse>> {
    register
        .book_repository()
        .find_by_id(book_id)
        .await
        .and_then(|bc| match bc {
            Some(bc) => Ok(Json(BookResponse::from(bc))),
            None => Err(AppError::EntityNotFound("not found".to_string())),
        })
}
