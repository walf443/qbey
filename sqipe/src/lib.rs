/// Value represents a bind parameter value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Int(n as i64)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Int(n)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Float(n)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

/// Comparison operator.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
}

impl Op {
    fn as_str(&self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Ne => "!=",
            Op::Gt => ">",
            Op::Lt => "<",
            Op::Gte => ">=",
            Op::Lte => "<=",
        }
    }
}

/// A column reference used to build conditions and order-by clauses.
#[derive(Debug, Clone)]
pub struct Col {
    name: String,
}

/// Create a column reference.
pub fn col(name: &str) -> Col {
    Col {
        name: name.to_string(),
    }
}

impl Col {
    pub fn eq(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition {
            col: self.name,
            op: Op::Eq,
            val: val.into(),
        }
    }

    pub fn ne(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition {
            col: self.name,
            op: Op::Ne,
            val: val.into(),
        }
    }

    pub fn gt(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition {
            col: self.name,
            op: Op::Gt,
            val: val.into(),
        }
    }

    pub fn lt(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition {
            col: self.name,
            op: Op::Lt,
            val: val.into(),
        }
    }

    pub fn gte(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition {
            col: self.name,
            op: Op::Gte,
            val: val.into(),
        }
    }

    pub fn lte(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition {
            col: self.name,
            op: Op::Lte,
            val: val.into(),
        }
    }

    pub fn asc(self) -> OrderByClause {
        OrderByClause {
            col: self.name,
            dir: SortDir::Asc,
        }
    }

    pub fn desc(self) -> OrderByClause {
        OrderByClause {
            col: self.name,
            dir: SortDir::Desc,
        }
    }
}

/// A WHERE condition tree.
#[derive(Debug, Clone)]
pub enum WhereClause {
    Condition { col: String, op: Op, val: Value },
    Any(Vec<WhereClause>),
    All(Vec<WhereClause>),
}

/// Tuple shorthand: `("name", value)` becomes `col = value`.
impl<V: Into<Value>> From<(&str, V)> for WhereClause {
    fn from((col, val): (&str, V)) -> Self {
        WhereClause::Condition {
            col: col.to_string(),
            op: Op::Eq,
            val: val.into(),
        }
    }
}

/// Combine conditions with OR: `any(a, b)` => `(a OR b)`.
pub fn any(a: impl Into<WhereClause>, b: impl Into<WhereClause>) -> WhereClause {
    WhereClause::Any(vec![a.into(), b.into()])
}

/// Combine conditions with AND: `all(a, b)` => `(a AND b)`.
pub fn all(a: impl Into<WhereClause>, b: impl Into<WhereClause>) -> WhereClause {
    WhereClause::All(vec![a.into(), b.into()])
}

#[derive(Debug, Clone)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct OrderByClause {
    col: String,
    dir: SortDir,
}

/// Trait for SQL dialect placeholder and quoting styles.
pub trait Dialect {
    fn placeholder(&self, index: usize) -> String;

    fn quote_identifier(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}

#[derive(Debug, Clone)]
enum WhereEntry {
    And(WhereClause),
    Or(WhereClause),
}

/// Internal config for SQL generation.
struct SqlConfig<'a> {
    ph: &'a dyn Fn(usize) -> String,
    qi: &'a dyn Fn(&str) -> String,
}

/// Default double-quote identifier quoting (SQL standard).
fn default_quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The query builder.
#[derive(Debug, Clone)]
pub struct Query {
    table: String,
    selects: Vec<String>,
    wheres: Vec<WhereEntry>,
    order_bys: Vec<OrderByClause>,
    limit_val: Option<u64>,
    offset_val: Option<u64>,
}

/// Create a new query builder for the given table.
pub fn sqipe(table: &str) -> Query {
    Query {
        table: table.to_string(),
        selects: Vec::new(),
        wheres: Vec::new(),
        order_bys: Vec::new(),
        limit_val: None,
        offset_val: None,
    }
}

impl Query {
    pub fn and_where(&mut self, cond: impl Into<WhereClause>) -> &mut Self {
        self.wheres.push(WhereEntry::And(cond.into()));
        self
    }

    pub fn or_where(&mut self, cond: impl Into<WhereClause>) -> &mut Self {
        self.wheres.push(WhereEntry::Or(cond.into()));
        self
    }

    pub fn select(&mut self, cols: &[&str]) -> &mut Self {
        self.selects = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn order_by(&mut self, clause: OrderByClause) -> &mut Self {
        self.order_bys.push(clause);
        self
    }

    pub fn limit(&mut self, n: u64) -> &mut Self {
        self.limit_val = Some(n);
        self
    }

    pub fn offset(&mut self, n: u64) -> &mut Self {
        self.offset_val = Some(n);
        self
    }

    /// Build standard SQL with `?` placeholders and double-quote identifiers.
    pub fn to_sql(&self) -> (String, Vec<Value>) {
        let cfg = SqlConfig {
            ph: &|_| "?".to_string(),
            qi: &default_quote_identifier,
        };
        self.build_standard_sql(&cfg)
    }

    /// Build pipe syntax SQL with `?` placeholders and double-quote identifiers.
    pub fn to_pipe_sql(&self) -> (String, Vec<Value>) {
        let cfg = SqlConfig {
            ph: &|_| "?".to_string(),
            qi: &default_quote_identifier,
        };
        self.build_pipe_sql(&cfg)
    }

    /// Build standard SQL with dialect-specific placeholders and quoting.
    pub fn to_sql_with(&self, dialect: &dyn Dialect) -> (String, Vec<Value>) {
        let cfg = SqlConfig {
            ph: &|n| dialect.placeholder(n),
            qi: &|name| dialect.quote_identifier(name),
        };
        self.build_standard_sql(&cfg)
    }

    /// Build pipe syntax SQL with dialect-specific placeholders and quoting.
    pub fn to_pipe_sql_with(&self, dialect: &dyn Dialect) -> (String, Vec<Value>) {
        let cfg = SqlConfig {
            ph: &|n| dialect.placeholder(n),
            qi: &|name| dialect.quote_identifier(name),
        };
        self.build_pipe_sql(&cfg)
    }

    fn build_select_clause(&self, cfg: &SqlConfig) -> String {
        if self.selects.is_empty() {
            "SELECT *".to_string()
        } else {
            let cols: Vec<String> = self.selects.iter().map(|c| (cfg.qi)(c)).collect();
            format!("SELECT {}", cols.join(", "))
        }
    }

    fn build_order_by_clause(&self, cfg: &SqlConfig) -> Option<String> {
        if self.order_bys.is_empty() {
            return None;
        }
        let clauses: Vec<String> = self
            .order_bys
            .iter()
            .map(|o| {
                let dir = match o.dir {
                    SortDir::Asc => "ASC",
                    SortDir::Desc => "DESC",
                };
                format!("{} {}", (cfg.qi)(&o.col), dir)
            })
            .collect();
        Some(format!("ORDER BY {}", clauses.join(", ")))
    }

    fn build_limit_offset(&self) -> (Option<String>, Option<String>) {
        (
            self.limit_val.map(|n| format!("LIMIT {}", n)),
            self.offset_val.map(|n| format!("OFFSET {}", n)),
        )
    }

    fn build_standard_sql(&self, cfg: &SqlConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut parts = Vec::new();

        parts.push(self.build_select_clause(cfg));
        parts.push(format!("FROM {}", (cfg.qi)(&self.table)));

        if let Some(where_sql) = self.build_where(cfg, &mut binds) {
            parts.push(format!("WHERE {}", where_sql));
        }

        if let Some(order_by) = self.build_order_by_clause(cfg) {
            parts.push(order_by);
        }

        let (limit, offset) = self.build_limit_offset();
        if let Some(l) = limit {
            parts.push(l);
        }
        if let Some(o) = offset {
            parts.push(o);
        }

        (parts.join(" "), binds)
    }

    fn build_pipe_sql(&self, cfg: &SqlConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut parts = Vec::new();

        parts.push(format!("FROM {}", (cfg.qi)(&self.table)));

        if let Some(where_sql) = self.build_where(cfg, &mut binds) {
            parts.push(format!("WHERE {}", where_sql));
        }

        parts.push(self.build_select_clause(cfg));

        if let Some(order_by) = self.build_order_by_clause(cfg) {
            parts.push(order_by);
        }

        let (limit, offset) = self.build_limit_offset();
        let mut limit_offset_parts = Vec::new();
        if let Some(l) = limit {
            limit_offset_parts.push(l);
        }
        if let Some(o) = offset {
            limit_offset_parts.push(o);
        }
        if !limit_offset_parts.is_empty() {
            parts.push(limit_offset_parts.join(" "));
        }

        (parts.join(" |> "), binds)
    }

    fn build_where(&self, cfg: &SqlConfig, binds: &mut Vec<Value>) -> Option<String> {
        if self.wheres.is_empty() {
            return None;
        }

        let single = self.wheres.len() == 1;
        let mut sql = String::new();

        for (i, entry) in self.wheres.iter().enumerate() {
            let (connector, clause) = match entry {
                WhereEntry::And(c) => ("AND", c),
                WhereEntry::Or(c) => ("OR", c),
            };

            if i > 0 {
                sql.push_str(&format!(" {} ", connector));
            }

            let is_top_level = single;
            sql.push_str(&render_where_clause(clause, is_top_level, cfg, binds));
        }

        Some(sql)
    }
}

fn render_where_clause(
    clause: &WhereClause,
    is_top_level: bool,
    cfg: &SqlConfig,
    binds: &mut Vec<Value>,
) -> String {
    match clause {
        WhereClause::Condition { col, op, val } => {
            binds.push(val.clone());
            let placeholder = (cfg.ph)(binds.len());
            format!("{} {} {}", (cfg.qi)(col), op.as_str(), placeholder)
        }
        WhereClause::Any(clauses) => {
            let parts: Vec<String> = clauses
                .iter()
                .map(|c| render_where_clause(c, false, cfg, binds))
                .collect();
            let joined = parts.join(" OR ");
            if is_top_level {
                joined
            } else {
                format!("({})", joined)
            }
        }
        WhereClause::All(clauses) => {
            let parts: Vec<String> = clauses
                .iter()
                .map(|c| render_where_clause(c, false, cfg, binds))
                .collect();
            let joined = parts.join(" AND ");
            if is_top_level {
                joined
            } else {
                format!("({})", joined)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_select_to_sql() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, binds) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"name\" = ?"
        );
        assert_eq!(binds, vec![Value::String("Alice".to_string())]);
    }

    #[test]
    fn test_basic_select_to_pipe_sql() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.select(&["id", "name"]);

        let (sql, _binds) = q.to_pipe_sql();
        assert_eq!(
            sql,
            "FROM \"employee\" |> WHERE \"name\" = ? |> SELECT \"id\", \"name\""
        );
    }

    #[test]
    fn test_select_star_when_no_select() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));

        let (sql, _) = q.to_sql();
        assert_eq!(sql, "SELECT * FROM \"employee\" WHERE \"name\" = ?");
    }

    #[test]
    fn test_comparison_operators() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.and_where(col("age").gt(20));
        q.and_where(col("age").lte(60));
        q.and_where(col("salary").lt(100000));
        q.and_where(col("level").gte(3));
        q.and_where(col("role").ne("intern"));
        q.select(&["id", "name"]);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"name\" = ? AND \"age\" > ? AND \"age\" <= ? AND \"salary\" < ? AND \"level\" >= ? AND \"role\" != ?"
        );
    }

    #[test]
    fn test_or_where() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.or_where(col("role").eq("admin"));

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM \"employee\" WHERE \"name\" = ? OR \"role\" = ?"
        );
    }

    #[test]
    fn test_any_grouping() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.and_where(any(col("role").eq("admin"), col("role").eq("manager")));

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM \"employee\" WHERE \"name\" = ? AND (\"role\" = ? OR \"role\" = ?)"
        );
    }

    #[test]
    fn test_any_all_combined() {
        let mut q = sqipe("employee");
        q.and_where(any(
            all(col("role").eq("admin"), col("dept").eq("eng")),
            all(col("role").eq("manager"), col("dept").eq("sales")),
        ));

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM \"employee\" WHERE (\"role\" = ? AND \"dept\" = ?) OR (\"role\" = ? AND \"dept\" = ?)"
        );
    }

    #[test]
    fn test_order_by() {
        let mut q = sqipe("employee");
        q.select(&["id", "name", "age"]);
        q.order_by(col("name").asc());
        q.order_by(col("age").desc());

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\", \"age\" FROM \"employee\" ORDER BY \"name\" ASC, \"age\" DESC"
        );

        let (sql, _) = q.to_pipe_sql();
        assert_eq!(
            sql,
            "FROM \"employee\" |> SELECT \"id\", \"name\", \"age\" |> ORDER BY \"name\" ASC, \"age\" DESC"
        );
    }

    #[test]
    fn test_limit_offset() {
        let mut q = sqipe("employee");
        q.select(&["id", "name"]);
        q.limit(10);
        q.offset(20);

        let (sql, _) = q.to_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" LIMIT 10 OFFSET 20"
        );

        let (sql, _) = q.to_pipe_sql();
        assert_eq!(
            sql,
            "FROM \"employee\" |> SELECT \"id\", \"name\" |> LIMIT 10 OFFSET 20"
        );
    }

    #[test]
    fn test_method_chaining() {
        let (sql, _) = sqipe("employee")
            .and_where(("name", "Alice"))
            .and_where(col("age").gt(20))
            .select(&["id", "name"])
            .to_sql();

        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"name\" = ? AND \"age\" > ?"
        );
    }

    #[test]
    fn test_binds_order() {
        let mut q = sqipe("employee");
        q.and_where(("name", "Alice"));
        q.and_where(col("age").gt(20));

        let (_, binds) = q.to_sql();
        assert_eq!(
            binds,
            vec![Value::String("Alice".to_string()), Value::Int(20)]
        );
    }
}
