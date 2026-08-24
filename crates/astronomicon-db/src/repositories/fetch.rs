use crate::error::DbResult;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, SqlitePool};

pub async fn fetch_optional_by_param<'a, T, P>(
    pool: &SqlitePool,
    query: &'a str,
    param: P,
) -> DbResult<Option<T>>
where
    T: for<'r> FromRow<'r, SqliteRow> + Send + Unpin,
    P: sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'a,
{
    let item = sqlx::query_as::<_, T>(query)
        .bind(param)
        .fetch_optional(pool)
        .await?;
    Ok(item)
}

pub async fn fetch_all<T>(pool: &SqlitePool, query: &str) -> DbResult<Vec<T>>
where
    T: for<'r> FromRow<'r, SqliteRow> + Send + Unpin,
{
    let items = sqlx::query_as::<_, T>(query).fetch_all(pool).await?;
    Ok(items)
}

pub async fn fetch_all_by_param<'a, T, P>(
    pool: &SqlitePool,
    query: &'a str,
    param: P,
) -> DbResult<Vec<T>>
where
    T: for<'r> FromRow<'r, SqliteRow> + Send + Unpin,
    P: sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'a,
{
    let items = sqlx::query_as::<_, T>(query)
        .bind(param)
        .fetch_all(pool)
        .await?;
    Ok(items)
}