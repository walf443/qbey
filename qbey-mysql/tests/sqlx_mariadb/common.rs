use sqlx::MySqlPool;
use std::sync::atomic::Ordering::Relaxed;
use testcontainers::runners::AsyncRunner;

// NOTE: The same macro is defined in `qbey/tests/common/mod.rs`.
// Keep both in sync when making changes.
macro_rules! define_shared_container {
    ($image:ty, $port:expr) => {
        struct SharedContainer {
            container: std::sync::Mutex<Option<testcontainers::ContainerAsync<$image>>>,
            host_port: u16,
        }

        static SHARED_CONTAINER: tokio::sync::OnceCell<SharedContainer> =
            tokio::sync::OnceCell::const_new();
        static DB_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        // Avoid unwrap() in dtor — panicking in a destructor causes process abort.
        // Errors are intentionally ignored since cleanup is best-effort.
        #[dtor::dtor(unsafe)]
        fn cleanup() {
            if let Some(shared) = SHARED_CONTAINER.get() {
                if let Some(container) = shared.container.lock().ok().and_then(|mut g| g.take()) {
                    if let Ok(rt) = tokio::runtime::Runtime::new() {
                        rt.block_on(async {
                            let _ = container.rm().await;
                        });
                    }
                }
            }
        }

        async fn get_shared_container() -> &'static SharedContainer {
            SHARED_CONTAINER
                .get_or_init(|| async {
                    let container = <$image>::default().start().await.unwrap();
                    let host_port = container.get_host_port_ipv4($port).await.unwrap();
                    SharedContainer {
                        container: std::sync::Mutex::new(Some(container)),
                        host_port,
                    }
                })
                .await
        }
    };
}

define_shared_container!(Mariadb, 3306);

/// Custom value type for MariaDB — maps directly to sqlx bind types.
#[derive(Debug, Clone)]
pub enum MysqlValue {
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Blob(Vec<u8>),
}

impl From<&str> for MysqlValue {
    fn from(s: &str) -> Self {
        MysqlValue::Text(s.to_string())
    }
}

impl From<i32> for MysqlValue {
    fn from(n: i32) -> Self {
        MysqlValue::Int(n as i64)
    }
}

impl From<f64> for MysqlValue {
    fn from(n: f64) -> Self {
        MysqlValue::Float(n)
    }
}

impl From<bool> for MysqlValue {
    fn from(b: bool) -> Self {
        MysqlValue::Bool(b)
    }
}

impl From<String> for MysqlValue {
    fn from(s: String) -> Self {
        MysqlValue::Text(s)
    }
}

impl From<Vec<u8>> for MysqlValue {
    fn from(b: Vec<u8>) -> Self {
        MysqlValue::Blob(b)
    }
}

pub async fn setup_pool() -> MySqlPool {
    let shared = get_shared_container().await;
    let db_id = DB_COUNTER.fetch_add(1, Relaxed);
    let db_name = format!("test_{}", db_id);

    let root_url = format!("mysql://root@127.0.0.1:{}", shared.host_port);
    let root_pool = MySqlPool::connect(&root_url).await.unwrap();

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE `{}`",
        db_name
    )))
    .execute(&root_pool)
    .await
    .unwrap();

    let url = format!("mysql://root@127.0.0.1:{}/{}", shared.host_port, db_name);
    let pool = MySqlPool::connect(&url).await.unwrap();

    sqlx::query(
        "CREATE TABLE users (
            id INT PRIMARY KEY AUTO_INCREMENT,
            name VARCHAR(255) NOT NULL,
            age INT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35)")
        .execute(&pool)
        .await
        .unwrap();

    pool
}

pub fn bind_params<'a>(
    mut query: sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    binds: &'a [MysqlValue],
) -> sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    for bind in binds {
        query = match bind {
            MysqlValue::Text(s) => query.bind(s.as_str()),
            MysqlValue::Int(n) => query.bind(*n),
            MysqlValue::Float(f) => query.bind(*f),
            MysqlValue::Bool(b) => query.bind(*b),
            MysqlValue::Blob(b) => query.bind(b.as_slice()),
        };
    }
    query
}

/// Minimal `mariadb` image definition.
///
/// Replaces `testcontainers_modules::mariadb::Mariadb` so that this crate does not depend on
/// the `testcontainers-modules` release cycle.
///
/// The default tag is the oldest MariaDB release still under upstream maintenance —
/// i.e. the minimum version this crate supports. It is deliberately NOT the
/// latest release and should only move when the supported range changes. CI
/// overrides `QBEY_TEST_MARIADB_TAG` to additionally run against the newest LTS.
#[derive(Debug, Clone)]
pub struct Mariadb {
    tag: String,
}

impl Default for Mariadb {
    fn default() -> Self {
        Self {
            tag: std::env::var("QBEY_TEST_MARIADB_TAG").unwrap_or_else(|_| "10.11".to_owned()),
        }
    }
}

impl testcontainers::Image for Mariadb {
    fn name(&self) -> &str {
        "mariadb"
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![
            testcontainers::core::WaitFor::message_on_stderr("mariadbd: ready for connections."),
            testcontainers::core::WaitFor::message_on_stderr("port: 3306"),
        ]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<
        Item = (
            impl Into<std::borrow::Cow<'_, str>>,
            impl Into<std::borrow::Cow<'_, str>>,
        ),
    > {
        [
            ("MARIADB_DATABASE", "test"),
            ("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "1"),
        ]
    }
}
