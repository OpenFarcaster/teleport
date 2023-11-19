use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use teleport_common::errors::HubError;

pub const DB_DIRECTORY: &str = "./.store";
pub const MAX_DB_ITERATOR_OPEN_MS: u64 = 60 * 1000;

const DB_NAME_DEFAULT: &str = "farcaster";

pub struct Store {
    conn: SqlitePool,
    name: String,
}

impl Store {
    pub async fn new(name: String) -> Self {
        let conn = SqlitePool::connect(&name).await.unwrap();
        Store { conn, name }
    }
}

fn get_db_path(name: &str) -> String {
    format!("sqlite:{}/{}", DB_DIRECTORY, name)
}

#[cfg(test)]
mod tests {
    use sqlx::migrate::MigrateDatabase;

    use super::*;
    use std::fs;

    #[test]
    fn test_get_db_path() {
        assert_eq!(get_db_path("test"), ".store/test");
    }

    #[tokio::test]
    async fn test_create_new_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_name = format!("sqlite:{}", db_path.to_str().unwrap());
        sqlx::Sqlite::create_database(&db_name).await.unwrap();

        println!("db_name: {}", db_name);
        let store = Store::new(db_name).await;
        let mut conn = store.conn.acquire().await.unwrap();
        let test_query = r#"CREATE TABLE IF NOT EXISTS test (
                   id INTEGER PRIMARY KEY,
                   name TEXT NOT NULL
        )"#;
        sqlx::query(test_query).execute(&mut *conn).await.unwrap();

        let test_insert_query = r#"INSERT INTO test (id, name) VALUES (?, ?)"#;
        sqlx::query(test_insert_query)
            .bind(1)
            .bind("test 1")
            .execute(&mut *conn)
            .await
            .unwrap();

        let test_select_query = r#"SELECT id, name FROM test"#;
        let row = sqlx::query(test_select_query)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        let id: i64 = row.get(0);
        let name: String = row.get(1);
        assert_eq!(id, 1);
        assert_eq!(name, "test 1");
    }
}
