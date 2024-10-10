use axum::Router;
use registry::AppRegistry;

use super::{book::build_book_routes, health::build_health_check_routes, user::build_user_routes};

pub fn routes() -> Router<AppRegistry> {
    let router = Router::new()
        .merge(build_health_check_routes())
        .merge(build_user_routes())
        .merge(build_book_routes());

    Router::new().nest("/api/v1", router)
}
