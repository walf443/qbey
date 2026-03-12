use std::ops::{Deref, DerefMut};

struct BigQuery;

impl sqipe::Dialect for BigQuery {
    fn placeholder(&self, index: usize) -> String {
        format!("@p{}", index)
    }
}

/// BigQuery-specific query builder wrapping the core Query.
pub struct BigQueryQuery {
    inner: sqipe::Query,
}

impl Deref for BigQueryQuery {
    type Target = sqipe::Query;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BigQueryQuery {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Create a BigQuery-specific query builder for the given table.
pub fn sqipe(table: &str) -> BigQueryQuery {
    BigQueryQuery {
        inner: sqipe::sqipe(table),
    }
}

impl BigQueryQuery {
    /// Build standard SQL with BigQuery dialect.
    pub fn to_sql(&self) -> (String, Vec<sqipe::Value>) {
        self.inner.to_sql_with(&BigQuery)
    }

    /// Build pipe syntax SQL with BigQuery dialect.
    pub fn to_pipe_sql(&self) -> (String, Vec<sqipe::Value>) {
        self.inner.to_pipe_sql_with(&BigQuery)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqipe::col;

    #[test]
    fn test_basic_to_pipe_sql() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_pipe_sql();
        assert_eq!(sql, "FROM employee |> WHERE name = @p1 |> SELECT id, name");
    }

    #[test]
    fn test_basic_to_sql() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(sql, "SELECT id, name FROM employee WHERE name = @p1");
    }

    #[test]
    fn test_delegates_core_methods() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.and_where(col("age").gt(20));
        q.select(&["id", "name"]);
        q.order_by(col("name").asc());
        q.limit(10);
        q.offset(5);

        let (sql, _) = q.to_pipe_sql();
        assert_eq!(
            sql,
            "FROM employee |> WHERE name = @p1 AND age > @p2 |> SELECT id, name |> ORDER BY name ASC |> LIMIT 10 OFFSET 5"
        );
    }
}
