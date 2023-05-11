use sqlx::migrate::Migrator;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::path::Path;
use std::thread;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[derive(Debug)]
pub struct TestDb {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

impl TestDb {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        migration_path: impl Into<String>,
    ) -> Self {
        let user = user.into();
        let password = password.into();
        let host = host.into();

        let uuid = Uuid::new_v4();
        let dbname = format!("test_{}", uuid);
        let dbname_cloned = dbname.clone();

        let tdb = Self {
            host,
            port,
            user,
            password,
            dbname,
        };

        let server_url = tdb.server_url();

        let url = tdb.url();
        let migration_path = migration_path.into();
        let path = migration_path;

        // create database dbname
        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                // use another connection to create database
                let mut conn = PgConnection::connect(&server_url).await.unwrap();
                conn.execute(format!(r#"CREATE DATABASE "{}""#, dbname_cloned).as_str())
                    .await
                    .unwrap();

                // now connect to test database for migration
                let mut conn = PgConnection::connect(&url).await.unwrap();
                let m = Migrator::new(Path::new(&path)).await.unwrap();
                m.run(&mut conn).await.unwrap();
            });
        })
        .join()
        .expect("Failed to create database.");

        tdb
    }

    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!(
                "postgres://{}@{}:{}/{}",
                self.user, self.host, self.port, "postgres"
            )
        } else {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, "postgres"
            )
        }
    }

    pub fn url(&self) -> String {
        if self.password.is_empty() {
            format!(
                "postgres://{}@{}:{}/{}",
                self.user, self.host, self.port, self.dbname
            )
        } else {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, self.dbname
            )
        }
    }

    pub async fn get_pool(&self) -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url())
            .await
            .unwrap()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let server_url = self.server_url();
        let dbname = self.dbname.clone();
        println!("server_url: {}", server_url);
        println!("Dropping database: {}", dbname);

        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = sqlx::PgConnection::connect(&server_url).await.unwrap();

                // terminate existing connections
                sqlx::query(&format!(r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = '{}'"#, dbname))
                    .execute(&mut conn)
                    .await
                    .expect("Terminate all other connections");


                conn.execute(format!(r#"DROP DATABASE "{}""#, dbname).as_str())
                    .await
                    .expect("Error while querying the drop database")
            });
        })
        .join()
        .expect("Failed to drop database.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_should_create_and_drop() {
        let tdb = TestDb::new("127.0.0.1", 5432, "admin", "P@ssw0rd", "./migrations");
        let pool = tdb.get_pool().await;
        // insert todo
        sqlx::query("INSERT INTO todo (title) VALUES ('test')")
            .execute(&pool)
            .await
            .unwrap();
        // get todo
        let (id, title) = sqlx::query_as::<_, (i32, String)>("SELECT id, title FROM todo")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(title, "test");
    }
}
