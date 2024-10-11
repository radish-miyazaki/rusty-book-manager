use axum::{
    routing::{get, post, put},
    Router,
};

use registry::AppRegistry;

use crate::handler::{
    book::{delete_book, register_book, show_book, show_book_list, update_book},
    checkout::{checkout_book, checkout_history, return_book, show_checked_out_list},
};

pub fn build_book_routes() -> Router<AppRegistry> {
    let books_routers = Router::new()
        .route("/", post(register_book))
        .route("/", get(show_book_list))
        .route(
            "/:book_id",
            get(show_book).put(update_book).delete(delete_book),
        )
        .route("/checkouts", get(show_checked_out_list))
        .route("/:book_id/checkouts", post(checkout_book))
        .route(
            "/:book_id/checkouts/:checkout_id/returned",
            put(return_book),
        )
        .route("/:book_id/checkout-history", get(checkout_history));

    Router::new().nest("/books", books_routers)
}
