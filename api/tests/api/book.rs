use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rstest::rstest;
use tower::ServiceExt;

use crate::{
    deserialize_json,
    helper::{fixture, make_router, v1, TestRequestExt},
};
use api::model::book::PaginatedBookResponse;
use kernel::{
    model::{
        book::Book,
        id::{BookId, UserId},
        list::PaginatedList,
        user::BookOwner,
    },
    repository::book::MockBookRepository,
};

#[rstest]
#[case("/books", 20, 0)]
#[case("/books?limit=50", 50, 0)]
#[case("/books?limit=50&offset=20", 50, 20)]
#[case("/books?offset=20", 20, 20)]
#[tokio::test]
async fn show_book_list_with_query_200(
    mut fixture: registry::MockAppRegistryExt,
    #[case] path: &str,
    #[case] expected_limit: i64,
    #[case] expected_offset: i64,
) -> anyhow::Result<()> {
    let book_id = BookId::new();

    fixture.expect_book_repository().returning(move || {
        let mut mock = MockBookRepository::new();
        mock.expect_find_all().returning(move |opt| {
            let items = vec![Book {
                id: book_id,
                title: "Rust による Web アプリケーション開発".to_string(),
                isbn: "".to_string(),
                author: "Yuki Toyoda".to_string(),
                description: "Rust による Web アプリケーション開発".to_string(),
                owner: BookOwner {
                    id: UserId::new(),
                    name: "radish-miyazaki".to_string(),
                },
                checkout: None,
            }];

            Ok(PaginatedList {
                total: 1,
                limit: opt.limit,
                offset: opt.offset,
                items,
            })
        });

        Arc::new(mock)
    });

    let app: axum::Router = make_router(fixture);

    let req = Request::get(&v1(path)).bearer().body(Body::empty())?;
    let resp = app.oneshot(req).await?;
    assert_eq!(resp.status(), StatusCode::OK);

    let result = deserialize_json!(resp, PaginatedBookResponse);
    assert_eq!(result.limit, expected_limit);
    assert_eq!(result.offset, expected_offset);

    Ok(())
}

#[rstest]
#[case("/books?limit=-1")]
#[case("/books?offset=aaa")]
#[tokio::test]
async fn show_book_list_with_query_400(
    mut fixture: registry::MockAppRegistryExt,
    #[case] path: &str,
) -> anyhow::Result<()> {
    let book_id = BookId::new();

    fixture.expect_book_repository().returning(move || {
        let mut mock = MockBookRepository::new();
        mock.expect_find_all().returning(move |opt| {
            let items = vec![Book {
                id: book_id,
                title: "Rust による Web アプリケーション開発".to_string(),
                isbn: "".to_string(),
                author: "Yuki Toyoda".to_string(),
                description: "Rust による Web アプリケーション開発".to_string(),
                owner: BookOwner {
                    id: UserId::new(),
                    name: "radish-miyazaki".to_string(),
                },
                checkout: None,
            }];

            Ok(PaginatedList {
                total: 1,
                limit: opt.limit,
                offset: opt.offset,
                items,
            })
        });

        Arc::new(mock)
    });

    let app: axum::Router = make_router(fixture);
    let req = Request::get(&v1(path)).bearer().body(Body::empty())?;
    let resp = app.oneshot(req).await?;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    Ok(())
}
