use qbey::Dialect;
use qbey::Value;
use qbey::{DeleteQueryBuilder, MySqlDialect};

/// MySQL-specific DELETE query builder.
///
/// Extends the core `DeleteQuery` with MySQL-specific features like
/// `ORDER BY` and `LIMIT` in DELETE statements.
#[derive(Debug, Clone)]
pub struct MysqlDeleteQuery<V: Clone + std::fmt::Debug = Value> {
    inner: qbey::DeleteQuery<V>,
    order_bys: Vec<qbey::OrderByClause<qbey::Value>>,
    limit_val: Option<u64>,
}

impl<V: Clone + std::fmt::Debug> MysqlDeleteQuery<V> {
    pub(crate) fn new(inner: qbey::DeleteQuery<V>) -> Self {
        MysqlDeleteQuery {
            inner,
            order_bys: Vec::new(),
            limit_val: None,
        }
    }
}

impl<V: Clone + std::fmt::Debug> DeleteQueryBuilder<V> for MysqlDeleteQuery<V> {
    fn and_where(&mut self, cond: impl qbey::IntoWhereClause<V>) -> &mut Self {
        self.inner.and_where(cond);
        self
    }

    fn or_where(&mut self, cond: impl qbey::IntoWhereClause<V>) -> &mut Self {
        self.inner.or_where(cond);
        self
    }

    fn allow_without_where(&mut self) -> &mut Self {
        self.inner.allow_without_where();
        self
    }

    fn with_cte(
        &mut self,
        name: &str,
        columns: &[&str],
        query: impl qbey::IntoSelectTree<V>,
    ) -> &mut Self {
        self.inner.with_cte(name, columns, query);
        self
    }

    fn with_recursive_cte(
        &mut self,
        name: &str,
        columns: &[&str],
        query: impl qbey::IntoSelectTree<V>,
    ) -> &mut Self {
        self.inner.with_recursive_cte(name, columns, query);
        self
    }
}

impl<V: Clone + std::fmt::Debug> MysqlDeleteQuery<V> {
    /// Add an ORDER BY clause (MySQL extension).
    pub fn order_by(&mut self, clause: qbey::OrderByClause) -> &mut Self {
        self.order_bys.push(clause);
        self
    }

    /// Add a raw SQL expression to the ORDER BY clause (MySQL extension).
    pub fn order_by_expr(&mut self, raw: qbey::RawSql) -> &mut Self {
        self.order_bys.push(qbey::OrderByClause::Expr(raw));
        self
    }

    /// Set the LIMIT value (MySQL extension).
    pub fn limit(&mut self, n: u64) -> &mut Self {
        self.limit_val = Some(n);
        self
    }

    /// Add columns to the RETURNING clause (MariaDB 10.5+ extension).
    ///
    /// Not supported by MySQL. Requires MariaDB 10.5 or later.
    ///
    /// ```
    /// use qbey::{col, Value, ConditionExpr};
    /// use qbey_mysql::qbey;
    /// use qbey::DeleteQueryBuilder;
    ///
    /// let mut d = qbey("users").into_delete();
    /// d.and_where(col("id").eq(1));
    /// d.returning(&[col("id"), col("name")]);
    /// let (sql, _) = d.to_sql();
    /// assert_eq!(
    ///     sql,
    ///     "DELETE FROM `users` WHERE `id` = ? RETURNING `id`, `name`"
    /// );
    /// ```
    #[cfg(feature = "returning")]
    pub fn returning(&mut self, cols: &[qbey::Col]) -> &mut Self {
        self.inner.returning(cols);
        self
    }

    /// Build standard SQL with MySQL dialect.
    pub fn to_sql(&self) -> (String, Vec<V>) {
        let mut tree = self.inner.to_tree();
        let ph = |_: usize| "?".to_string();
        let qi = |name: &str| MySqlDialect.quote_identifier(name);
        let cfg = qbey::renderer::RenderConfig::from_dialect(&ph, &qi, &MySqlDialect);

        // MySQL-specific ORDER BY and LIMIT are inserted before the
        // RETURNING token (if present) so that the final SQL order is:
        // DELETE FROM ... WHERE ... ORDER BY ... LIMIT ... RETURNING ...
        let mut extra_tokens: Vec<qbey::tree::DeleteToken<V>> = Vec::new();

        let mut order_by_binds: Vec<Value> = Vec::new();
        if let Some(order_by) =
            qbey::renderer::render_order_by(&self.order_bys, &cfg, &mut order_by_binds)
        {
            debug_assert!(
                order_by_binds.is_empty(),
                "RawSql binds in MySQL DELETE ORDER BY are not supported with custom value types"
            );
            extra_tokens.push(qbey::tree::DeleteToken::Raw(order_by));
        }
        if let Some(n) = self.limit_val {
            extra_tokens.push(qbey::tree::DeleteToken::Raw(format!("LIMIT {}", n)));
        }

        if !extra_tokens.is_empty() {
            // Find the position of the Returning token (if any) and insert before it.
            #[cfg(feature = "returning")]
            let insert_pos = tree
                .tokens
                .iter()
                .position(|t| matches!(t, qbey::tree::DeleteToken::Returning(_)))
                .unwrap_or(tree.tokens.len());
            #[cfg(not(feature = "returning"))]
            let insert_pos = tree.tokens.len();

            for (i, token) in extra_tokens.into_iter().enumerate() {
                tree.tokens.insert(insert_pos + i, token);
            }
        }

        qbey::renderer::delete::render_delete(&tree, &cfg)
    }
}
