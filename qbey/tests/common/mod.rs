#![allow(dead_code)]

/// Defines `SharedContainer`, a static `OnceCell`, a `#[dtor::dtor]` cleanup
/// function, and `get_shared_container()` for the given testcontainers image
/// and port.
///
/// NOTE: The same macro is defined in `qbey-mysql/tests/sqlx_mysql/common.rs`.
/// Keep both in sync when making changes.
///
/// Usage:
/// ```ignore
/// define_shared_container!(Postgres, 5432);
/// ```
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

/// Minimal `mysql` image definition.
///
/// Replaces `testcontainers_modules::mysql::Mysql` so that this crate does not
/// depend on the `testcontainers-modules` release cycle.
#[derive(Debug, Default, Clone)]
pub struct Mysql;

impl testcontainers::Image for Mysql {
    fn name(&self) -> &str {
        "mysql"
    }

    fn tag(&self) -> &str {
        "8.1"
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![
            testcontainers::core::WaitFor::message_on_stderr(
                "X Plugin ready for connections. Bind-address",
            ),
            testcontainers::core::WaitFor::message_on_stderr(
                "/usr/sbin/mysqld: ready for connections.",
            ),
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
            ("MYSQL_DATABASE", "test"),
            ("MYSQL_ALLOW_EMPTY_PASSWORD", "yes"),
        ]
    }
}

/// Minimal `postgres` image definition.
///
/// Replaces `testcontainers_modules::postgres::Postgres` so that this crate does
/// not depend on the `testcontainers-modules` release cycle.
#[derive(Debug, Default, Clone)]
pub struct Postgres;

impl testcontainers::Image for Postgres {
    fn name(&self) -> &str {
        "postgres"
    }

    fn tag(&self) -> &str {
        "11-alpine"
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![
            testcontainers::core::WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ),
            testcontainers::core::WaitFor::message_on_stdout(
                "database system is ready to accept connections",
            ),
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
            ("POSTGRES_DB", "postgres"),
            ("POSTGRES_USER", "postgres"),
            ("POSTGRES_PASSWORD", "postgres"),
        ]
    }

    /// Matches the `testcontainers-modules` default: disable fsync for speed.
    fn cmd(&self) -> impl IntoIterator<Item = impl Into<std::borrow::Cow<'_, str>>> {
        ["-c", "fsync=off"]
    }
}
