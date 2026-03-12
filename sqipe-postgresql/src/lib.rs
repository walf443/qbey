use std::ops::{Deref, DerefMut};

struct PostgreSQL;

impl sqipe::Dialect for PostgreSQL {
    fn placeholder(&self, index: usize) -> String {
        format!("${}", index)
    }

    // PostgreSQL uses double quotes (same as default), but we implement explicitly for clarity.
    fn quote_identifier(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}

/// PostgreSQL-specific query builder wrapping the core Query.
pub struct PostgresqlQuery {
    inner: sqipe::Query,
}

impl Deref for PostgresqlQuery {
    type Target = sqipe::Query;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PostgresqlQuery {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Create a PostgreSQL-specific query builder for the given table.
pub fn sqipe(table: &str) -> PostgresqlQuery {
    PostgresqlQuery {
        inner: sqipe::sqipe(table),
    }
}

impl PostgresqlQuery {
    /// Build standard SQL with PostgreSQL dialect.
    pub fn to_sql(&self) -> (String, Vec<sqipe::Value>) {
        self.inner.to_sql_with(&PostgreSQL)
    }

    /// Build pipe syntax SQL with PostgreSQL dialect.
    pub fn to_pipe_sql(&self) -> (String, Vec<sqipe::Value>) {
        self.inner.to_pipe_sql_with(&PostgreSQL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqipe::col;

    #[test]
    fn test_basic_to_sql() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"name\" = $1"
        );
    }

    #[test]
    fn test_multiple_params() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.and_where(col("age").gt(20));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"name\" = $1 AND \"age\" > $2"
        );
    }

    #[test]
    fn test_delegates_core_methods() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);
        q.order_by(col("name").asc());
        q.limit(10);
        q.offset(5);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"name\" = $1 ORDER BY \"name\" ASC LIMIT 10 OFFSET 5"
        );
    }
}
