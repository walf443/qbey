use std::ops::{Deref, DerefMut};

struct MySQL;

impl sqipe::Dialect for MySQL {
    fn placeholder(&self, _index: usize) -> String {
        "?".to_string()
    }
}

/// MySQL-specific query builder wrapping the core Query.
pub struct MysqlQuery {
    inner: sqipe::Query,
    table: String,
    force_indexes: Vec<String>,
    use_indexes: Vec<String>,
    ignore_indexes: Vec<String>,
}

impl Deref for MysqlQuery {
    type Target = sqipe::Query;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MysqlQuery {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Create a MySQL-specific query builder for the given table.
pub fn sqipe(table: &str) -> MysqlQuery {
    MysqlQuery {
        inner: sqipe::sqipe(table),
        table: table.to_string(),
        force_indexes: Vec::new(),
        use_indexes: Vec::new(),
        ignore_indexes: Vec::new(),
    }
}

impl MysqlQuery {
    pub fn force_index(&mut self, indexes: &[&str]) -> &mut Self {
        self.force_indexes = indexes.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn use_index(&mut self, indexes: &[&str]) -> &mut Self {
        self.use_indexes = indexes.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn ignore_index(&mut self, indexes: &[&str]) -> &mut Self {
        self.ignore_indexes = indexes.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Build standard SQL with MySQL dialect.
    pub fn to_sql(&self) -> (String, Vec<sqipe::Value>) {
        let (sql, binds) = self.inner.to_sql_with(&MySQL);
        (self.inject_index_hints(sql), binds)
    }

    /// Build pipe syntax SQL with MySQL dialect.
    pub fn to_pipe_sql(&self) -> (String, Vec<sqipe::Value>) {
        let (sql, binds) = self.inner.to_pipe_sql_with(&MySQL);
        (self.inject_index_hints(sql), binds)
    }

    fn inject_index_hints(&self, sql: String) -> String {
        let hints = self.build_index_hints();
        if hints.is_empty() {
            return sql;
        }
        let from_table = format!("FROM {}", self.table);
        sql.replacen(&from_table, &format!("{} {}", from_table, hints), 1)
    }

    fn build_index_hints(&self) -> String {
        let mut parts = Vec::new();
        if !self.force_indexes.is_empty() {
            parts.push(format!("FORCE INDEX ({})", self.force_indexes.join(", ")));
        }
        if !self.use_indexes.is_empty() {
            parts.push(format!("USE INDEX ({})", self.use_indexes.join(", ")));
        }
        if !self.ignore_indexes.is_empty() {
            parts.push(format!("IGNORE INDEX ({})", self.ignore_indexes.join(", ")));
        }
        parts.join(" ")
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
        assert_eq!(sql, "SELECT id, name FROM employee WHERE name = ?");
    }

    #[test]
    fn test_force_index() {
        let mut q = sqipe("employee");
        q.force_index(&["idx_name"]);
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT id, name FROM employee FORCE INDEX (idx_name) WHERE name = ?"
        );
    }

    #[test]
    fn test_force_index_multiple() {
        let mut q = sqipe("employee");
        q.force_index(&["idx_name", "idx_age"]);
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT id, name FROM employee FORCE INDEX (idx_name, idx_age) WHERE name = ?"
        );
    }

    #[test]
    fn test_use_index() {
        let mut q = sqipe("employee");
        q.use_index(&["idx_name"]);
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT id, name FROM employee USE INDEX (idx_name) WHERE name = ?"
        );
    }

    #[test]
    fn test_ignore_index() {
        let mut q = sqipe("employee");
        q.ignore_index(&["idx_old"]);
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT id, name FROM employee IGNORE INDEX (idx_old) WHERE name = ?"
        );
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

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT id, name FROM employee WHERE name = ? AND age > ? ORDER BY name ASC LIMIT 10 OFFSET 5"
        );
    }

    #[test]
    fn test_force_index_pipe_sql() {
        let mut q = sqipe("employee");
        q.force_index(&["idx_name"]);
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_pipe_sql();
        assert_eq!(
            sql,
            "FROM employee FORCE INDEX (idx_name) |> WHERE name = ? |> SELECT id, name"
        );
    }
}
