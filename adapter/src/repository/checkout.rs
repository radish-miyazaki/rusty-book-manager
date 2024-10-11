use async_trait::async_trait;
use derive_new::new;
use kernel::{
    model::{
        checkout::{
            event::{CreateCheckout, UpdateReturned},
            Checkout,
        },
        id::{BookId, CheckoutId, UserId},
    },
    repository::checkout::CheckoutRepository,
};
use shared::error::{AppError, AppResult};

use crate::database::{
    model::checkout::{CheckoutRow, CheckoutStateRow, ReturnedCheckoutRow},
    ConnectionPool,
};

#[derive(new)]
pub struct CheckoutRepositoryImpl {
    db: ConnectionPool,
}

#[async_trait]
impl CheckoutRepository for CheckoutRepositoryImpl {
    async fn create(&self, event: CreateCheckout) -> AppResult<()> {
        let mut tx = self.db.begin().await?;

        self.set_transaction_serializable(&mut tx).await?;

        {
            let res = sqlx::query_as!(
                CheckoutStateRow,
                r#"
                    SELECT
                        b.book_id,
                        c.checkout_id AS "checkout_id?: CheckoutId",
                        NULL AS "user_id?: UserId"
                    FROM books AS b
                    LEFT OUTER JOIN checkouts AS c USING (book_id)
                    WHERE book_id = $1;
                "#,
                event.book_id as _
            )
            .fetch_optional(self.db.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;

            match res {
                // 指定した書籍が存在しない場合
                None => {
                    return Err(AppError::EntityNotFound(format!(
                        "Book not found: book_id={}",
                        event.book_id
                    )))
                }
                // 指定した書籍が既に貸し出されている場合
                Some(CheckoutStateRow {
                    checkout_id: Some(_),
                    ..
                }) => {
                    return Err(AppError::UnprocessableEntity(format!(
                        "Book already checked out: book_id={}",
                        event.book_id
                    )))
                }
                _ => {}
            }
        }

        let checkout_id = CheckoutId::new();
        let res = sqlx::query!(
            r#"
                INSERT INTO checkouts
                (checkout_id, book_id, user_id, checked_out_at)
                VALUES ($1, $2, $3, $4);
            "#,
            checkout_id as _,
            event.book_id as _,
            event.checked_out_by as _,
            event.checked_out_at
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() == 0 {
            return Err(AppError::NoRowsAffectedError(
                "No checkout record has been created".to_string(),
            ));
        }

        tx.commit().await.map_err(AppError::TransactionError)?;

        Ok(())
    }

    async fn update_returned(&self, event: UpdateReturned) -> AppResult<()> {
        let mut tx = self.db.begin().await?;

        self.set_transaction_serializable(&mut tx).await?;

        {
            let res = sqlx::query_as!(
                CheckoutStateRow,
                r#"
                    SELECT
                        b.book_id,
                        c.checkout_id AS "checkout_id?: CheckoutId",
                        c.user_id AS "user_id?: UserId"
                    FROM books AS b
                    LEFT OUTER JOIN checkouts AS c USING (book_id)
                    WHERE book_id = $1;
                "#,
                event.book_id as _
            )
            .fetch_optional(self.db.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;

            match res {
                // 指定した書籍が存在しない場合
                None => {
                    return Err(AppError::EntityNotFound(format!(
                        "Book not found: book_id={}",
                        event.book_id
                    )))
                }
                // 指定した書籍が貸し出されているが、貸出 ID またはユーザ ID が一致しない場合
                Some(CheckoutStateRow {
                    checkout_id: Some(c),
                    user_id: Some(u),
                    ..
                }) if (c, u) != (event.checkout_id, event.returned_by) => {
                    return Err(AppError::UnprocessableEntity(format!(
                        "Book not checked out by the user: book_id={}, checkout_id={}, user_id={}",
                        event.book_id, event.checkout_id, event.returned_by
                    )))
                }
                _ => {}
            }
        }

        let res = sqlx::query!(
            r#"
                INSERT INTO returned_checkouts
                (checkout_id, book_id, user_id, checked_out_at, returned_at)
                SELECT checkout_id, book_id, user_id, checked_out_at, $2
                FROM checkouts
                WHERE checkout_id = $1
            "#,
            event.checkout_id as _,
            event.returned_at as _,
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() == 0 {
            return Err(AppError::NoRowsAffectedError(
                "No returning record has been updated".to_string(),
            ));
        }

        let res = sqlx::query!(
            r#"
                DELETE FROM checkouts WHERE checkout_id = $1;
            "#,
            event.checkout_id as _
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() == 0 {
            return Err(AppError::NoRowsAffectedError(
                "No checkout record has been deleted".to_string(),
            ));
        }

        tx.commit().await.map_err(AppError::TransactionError)?;

        Ok(())
    }

    async fn find_unreturned_all(&self) -> AppResult<Vec<Checkout>> {
        sqlx::query_as!(
            CheckoutRow,
            r#"
                SELECT
                    c.checkout_id,
                    c.book_id,
                    c.user_id,
                    c.checked_out_at,
                    b.title,
                    b.author,
                    b.isbn
                FROM checkouts AS c
                INNER JOIN books AS b USING (book_id)
                ORDER BY c.checked_out_at ASC;
            "#,
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map(|rows| rows.into_iter().map(Checkout::from).collect())
        .map_err(AppError::SpecificOperationError)
    }

    async fn find_unreturned_by_user_id(&self, user_id: UserId) -> AppResult<Vec<Checkout>> {
        sqlx::query_as!(
            CheckoutRow,
            r#"
                SELECT
                    c.checkout_id,
                    c.book_id,
                    c.user_id,
                    c.checked_out_at,
                    b.title,
                    b.author,
                    b.isbn
                FROM checkouts AS c
                INNER JOIN books AS b USING (book_id)
                WHERE c.user_id = $1
                ORDER BY c.checked_out_at ASC;
            "#,
            user_id as _
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map(|rows| rows.into_iter().map(Checkout::from).collect())
        .map_err(AppError::SpecificOperationError)
    }

    async fn find_history_by_book_id(&self, book_id: BookId) -> AppResult<Vec<Checkout>> {
        // 未返却の貸出情報を取得する
        let checkout: Option<Checkout> = self.find_unreturned_by_book_id(book_id).await?;

        // 返却済みの貸出情報を取得する
        let mut checkout_histories = sqlx::query_as!(
            ReturnedCheckoutRow,
            r#"
                SELECT
                    rc.checkout_id,
                    rc.book_id,
                    rc.user_id,
                    rc.checked_out_at,
                    rc.returned_at,
                    b.title,
                    b.author,
                    b.isbn
                FROM returned_checkouts AS rc
                INNER JOIN books AS b USING (book_id)
                WHERE rc.book_id = $1
                ORDER BY rc.checked_out_at ASC;
            "#,
            book_id as _
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?
        .into_iter()
        .map(Checkout::from)
        .collect::<Vec<Checkout>>();

        // 貸出中の情報があれば先頭に追加する
        if let Some(co) = checkout {
            checkout_histories.insert(0, co);
        }

        Ok(checkout_histories)
    }
}

impl CheckoutRepositoryImpl {
    /// トランザクション分離レベルを SERIALIZABLE に設定することで結果の整合性を担保する内部メソッド。
    /// create や update_returned などのメソッドでトランザクションを利用する場合に呼び出だされる。
    /// @see https://www.postgresql.jp/document/16/html/transaction-iso.html
    async fn set_transaction_serializable(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> AppResult<()> {
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut **tx)
            .await
            .map_err(AppError::SpecificOperationError)?;

        Ok(())
    }

    async fn find_unreturned_by_book_id(&self, book_id: BookId) -> AppResult<Option<Checkout>> {
        let res = sqlx::query_as!(
            CheckoutRow,
            r#"
                SELECT
                    c.checkout_id,
                    c.book_id,
                    c.user_id,
                    c.checked_out_at,
                    b.title,
                    b.author,
                    b.isbn
                FROM checkouts AS c
                INNER JOIN books AS b USING (book_id)
                WHERE c.book_id = $1;
            "#,
            book_id as _
        )
        .fetch_optional(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?
        .map(Checkout::from);

        Ok(res)
    }
}
