pub mod db;

use std::path::Path;

use sqlx::sqlite::SqlitePool;

use teleport_commong::config::Config;

pub const DB_DIRECTORY: &str = ".";

#[derive(Debug, Clone)]
pub struct Store {
    pub conn: SqlitePool,
    pub config: Config,
}

impl Store {
    pub async fn new(config: &Config) -> Self {
        let path = config.db_path;
        let conn = SqlitePool::connect(&path).await.unwrap();

        Self { conn, config }
    }

    pub async fn migrate(&self) {
        let migrator = sqlx::migrate::Migrator::new(Path::new(self.config.db_migrations_path))
            .await
            .unwrap();
        migrator.run(&self.conn).await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::MigrateDatabase;
    use sqlx::Row;

    #[tokio::test]
    async fn test_create_new_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_name = format!("sqlite:{}", db_path.to_str().unwrap());
        sqlx::Sqlite::create_database(&db_name).await.unwrap();

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
