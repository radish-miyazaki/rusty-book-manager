use std::collections::HashMap;

use async_trait::async_trait;
use derive_new::new;
use kernel::{
    model::{
        book::{
            event::{CreateBook, DeleteBook, UpdateBook},
            Book, BookListOptions, Checkout,
        },
        id::{BookId, UserId},
        list::PaginatedList,
    },
    repository::book::BookRepository,
};
use shared::error::{AppError, AppResult};

use crate::database::{
    model::book::{BookCheckoutRow, BookRow, PaginatedBookRow},
    ConnectionPool,
};

#[derive(new)]
pub struct BookRepositoryImpl {
    db: ConnectionPool,
}

#[async_trait]
impl BookRepository for BookRepositoryImpl {
    async fn create(&self, event: CreateBook, user_id: UserId) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO books (title, author, isbn, description, user_id)
                VALUES ($1, $2, $3, $4, $5);
            "#,
            event.title,
            event.author,
            event.isbn,
            event.description,
            user_id as _
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(())
    }

    async fn find_all(&self, options: BookListOptions) -> AppResult<PaginatedList<Book>> {
        let BookListOptions { limit, offset } = options;

        let rows: Vec<PaginatedBookRow> = sqlx::query_as!(
            PaginatedBookRow,
            r#"
                SELECT COUNT(*) OVER() as "total!",
                    b.book_id AS id
                FROM books AS b
                ORDER BY b.created_at DESC
                LIMIT $1
                OFFSET $2;
            "#,
            limit,
            offset
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        let total = rows
            .first()
            .map(|r| r.total)
            // レコードが 0 件の場合は total が NULL になるため、unwrap_or_default() でデフォルト値を設定
            .unwrap_or_default();
        let book_ids = rows.into_iter().map(|r| r.id).collect::<Vec<BookId>>();

        let rows: Vec<BookRow> = sqlx::query_as!(
            BookRow,
            r#"
                SELECT
                    b.book_id AS book_id,
                    b.title AS title,
                    b.author AS author,
                    b.isbn AS isbn,
                    b.description AS description,
                    u.user_id AS owned_by,
                    u.name AS owner_name
                FROM books AS b
                INNER JOIN users AS u USING (user_id)
                WHERE b.book_id IN (SELECT * FROM UNNEST($1::uuid[]))
                ORDER BY b.created_at DESC
            "#,
            &book_ids as _
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        let book_ids = rows.iter().map(|book| book.book_id).collect::<Vec<_>>();
        let mut checkouts = self.find_checkouts(&book_ids).await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let checkout = checkouts.remove(&row.book_id);
                row.into_book(checkout)
            })
            .collect();

        Ok(PaginatedList {
            total,
            limit,
            offset,
            items,
        })
    }

    async fn find_by_id(&self, book_id: BookId) -> AppResult<Option<Book>> {
        let row: Option<BookRow> = sqlx::query_as!(
            BookRow,
            r#"
                SELECT
                    b.book_id,
                    b.title,
                    b.author,
                    b.isbn,
                    b.description,
                    u.user_id AS owned_by,
                    u.name AS owner_name
                FROM books AS b
                INNER JOIN users AS u USING (user_id)
                WHERE book_id = $1;
            "#,
            book_id as _
        )
        .fetch_optional(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        match row {
            Some(r) => {
                let checkout = self.find_checkouts(&[r.book_id]).await?.remove(&r.book_id);
                Ok(Some(r.into_book(checkout)))
            }
            None => Ok(None),
        }
    }

    async fn update(&self, event: UpdateBook) -> AppResult<()> {
        let res = sqlx::query!(
            r#"
                UPDATE books
                SET
                    title = $1,
                    author = $2,
                    isbn = $3,
                    description = $4
                WHERE book_id = $5
                AND user_id = $6;
            "#,
            event.title,
            event.author,
            event.isbn,
            event.description,
            event.book_id as _,
            event.requested_user as _
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() == 0 {
            return Err(AppError::EntityNotFound("Specified book not found".into()));
        }

        Ok(())
    }

    async fn delete(&self, event: DeleteBook) -> AppResult<()> {
        let res = sqlx::query!(
            r#"
                DELETE FROM books
                WHERE book_id = $1
                AND user_id = $2;
            "#,
            event.book_id as _,
            event.requested_user as _
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() == 0 {
            return Err(AppError::EntityNotFound("Specified book not found".into()));
        }

        Ok(())
    }
}

impl BookRepositoryImpl {
    async fn find_checkouts(&self, book_ids: &[BookId]) -> AppResult<HashMap<BookId, Checkout>> {
        let res = sqlx::query_as!(
            BookCheckoutRow,
            r#"
                SELECT
                    c.checkout_id,
                    c.book_id,
                    u.user_id,
                    u.name AS user_name,
                    c.checked_out_at
                FROM checkouts AS c
                INNER JOIN users AS u USING (user_id)
                WHERE book_id = ANY($1);
            "#,
            book_ids as _
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?
        .into_iter()
        .map(|checkout| (checkout.book_id, Checkout::from(checkout)))
        .collect();

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[sqlx::test(fixtures("common"))]
    async fn test_register_book(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = BookRepositoryImpl::new(ConnectionPool::new(pool));

        let book = CreateBook {
            title: "Test Title".into(),
            author: "Test Author".into(),
            isbn: "Test ISBN".into(),
            description: "Test Description".into(),
        };
        let user_id = UserId::from_str("2bbd820c-7a88-450c-b056-19dcbadd527d")?;

        repo.create(book, user_id).await?;

        let options = BookListOptions {
            limit: 20,
            offset: 0,
        };
        let res = repo.find_all(options).await?;
        assert_eq!(res.items.len(), 1);

        let book_id = res.items[0].id;
        let res = repo.find_by_id(book_id).await?;
        assert!(res.is_some());

        let Book {
            id,
            title,
            author,
            isbn,
            description,
            owner,
            ..
        } = res.unwrap();
        assert_eq!(id, book_id);
        assert_eq!(title, "Test Title");
        assert_eq!(author, "Test Author");
        assert_eq!(isbn, "Test ISBN");
        assert_eq!(description, "Test Description");
        assert_eq!(owner.id, user_id);
        assert_eq!(owner.name, "Eleazar Fig");

        Ok(())
    }

    #[sqlx::test(fixtures("common", "book"))]
    async fn test_update_book(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = BookRepositoryImpl::new(ConnectionPool::new(pool.clone()));

        let book_id = BookId::from_str("9890736e-a4e4-461a-a77d-eac3517ef11b")?;
        let book = repo.find_by_id(book_id).await?.unwrap();
        const NEW_AUTHOR: &str = "更新後の著者名";
        assert_ne!(book.author, NEW_AUTHOR);

        let update_book = UpdateBook {
            book_id: book.id,
            title: book.title,
            author: NEW_AUTHOR.into(), // このフィールドを変更
            isbn: book.isbn,
            description: book.description,
            requested_user: UserId::from_str("2bbd820c-7a88-450c-b056-19dcbadd527d").unwrap(),
        };
        repo.update(update_book).await.unwrap();

        let book = repo.find_by_id(book_id).await?.unwrap();
        assert_eq!(book.author, NEW_AUTHOR);

        Ok(())
    }
}
