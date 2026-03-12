use crate::{AggregateExpr, AggregateFunc, OrderByClause, SortDir, Value, WhereClause, WhereEntry};

/// Default double-quote identifier quoting (SQL standard).
pub fn default_quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// FROM clause with optional dialect-specific modifiers appended after the table name.
#[derive(Debug, Clone)]
pub struct FromClause {
    pub table: String,
    /// Raw SQL fragments appended after the table name (e.g., "FORCE INDEX (idx)").
    /// Dialect crates populate this via tree transformation.
    pub table_suffix: Vec<String>,
}

/// What the SELECT clause looks like.
#[derive(Debug, Clone)]
pub enum SelectClause {
    /// SELECT * or SELECT col1, col2, ...
    Columns(Vec<String>),
    /// Aggregate: SELECT group_cols..., agg_exprs...
    Aggregate {
        group_bys: Vec<String>,
        exprs: Vec<AggregateExpr>,
    },
}

/// AST for a single SELECT query.
#[derive(Debug, Clone)]
pub struct SelectTree {
    pub from: FromClause,
    pub(crate) wheres: Vec<WhereEntry>,
    pub select: SelectClause,
    pub order_bys: Vec<OrderByClause>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// AST for a UNION query.
#[derive(Debug, Clone)]
pub struct UnionTree {
    pub parts: Vec<(crate::SetOp, SelectTree)>,
    pub order_bys: Vec<OrderByClause>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Configuration for rendering SQL from trees.
pub struct RenderConfig<'a> {
    pub ph: &'a dyn Fn(usize) -> String,
    pub qi: &'a dyn Fn(&str) -> String,
}

// ── Build tree from Query ──

impl SelectTree {
    pub fn from_query(query: &crate::Query) -> Self {
        let select = if !query.aggregates.is_empty() {
            SelectClause::Aggregate {
                group_bys: query.group_bys.clone(),
                exprs: query.aggregates.clone(),
            }
        } else {
            SelectClause::Columns(query.selects.clone())
        };

        SelectTree {
            from: FromClause {
                table: query.table.clone(),
                table_suffix: Vec::new(),
            },
            wheres: query.wheres.clone(),
            select,
            order_bys: query.order_bys.clone(),
            limit: query.limit_val,
            offset: query.offset_val,
        }
    }
}

impl UnionTree {
    pub fn from_union_query(union: &crate::UnionQuery) -> Self {
        let parts = union
            .parts
            .iter()
            .map(|(op, q)| (op.clone(), SelectTree::from_query(q)))
            .collect();
        UnionTree {
            parts,
            order_bys: union.order_bys.clone(),
            limit: union.limit_val,
            offset: union.offset_val,
        }
    }
}

// ── Renderer trait ──

pub trait Renderer {
    fn render_select(&self, tree: &SelectTree, cfg: &RenderConfig) -> (String, Vec<Value>);
    fn render_union(&self, tree: &UnionTree, cfg: &RenderConfig) -> (String, Vec<Value>);
}

// ── StandardSqlRenderer ──

pub struct StandardSqlRenderer;

impl StandardSqlRenderer {
    fn render_core(&self, tree: &SelectTree, cfg: &RenderConfig, binds: &mut Vec<Value>) -> String {
        let mut parts = Vec::new();

        match &tree.select {
            SelectClause::Columns(cols) => {
                parts.push(render_select_columns(cols, cfg));
            }
            SelectClause::Aggregate { group_bys, exprs } => {
                let mut items = Vec::new();
                for col in group_bys {
                    items.push((cfg.qi)(col));
                }
                for expr in exprs {
                    items.push(render_aggregate_expr(expr, cfg));
                }
                parts.push(format!("SELECT {}", items.join(", ")));
            }
        }

        parts.push(render_from(&tree.from, cfg));

        if let Some(where_sql) = render_wheres(&tree.wheres, cfg, binds) {
            parts.push(format!("WHERE {}", where_sql));
        }

        if let SelectClause::Aggregate { group_bys, .. } = &tree.select
            && !group_bys.is_empty()
        {
            let cols: Vec<String> = group_bys.iter().map(|c| (cfg.qi)(c)).collect();
            parts.push(format!("GROUP BY {}", cols.join(", ")));
        }

        parts.join(" ")
    }

    fn render_union_part(
        &self,
        tree: &SelectTree,
        cfg: &RenderConfig,
        binds: &mut Vec<Value>,
    ) -> String {
        let mut sql = self.render_core(tree, cfg, binds);
        let has_extra = !tree.order_bys.is_empty() || tree.limit.is_some() || tree.offset.is_some();

        if has_extra {
            append_order_by(&mut sql, &tree.order_bys, cfg, " ");
            append_limit_offset_flat(&mut sql, tree.limit, tree.offset);
            sql = format!("({})", sql);
        }

        sql
    }
}

impl Renderer for StandardSqlRenderer {
    fn render_select(&self, tree: &SelectTree, cfg: &RenderConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut sql = self.render_core(tree, cfg, &mut binds);
        append_order_by(&mut sql, &tree.order_bys, cfg, " ");
        append_limit_offset_flat(&mut sql, tree.limit, tree.offset);
        (sql, binds)
    }

    fn render_union(&self, tree: &UnionTree, cfg: &RenderConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut sql = String::new();

        for (i, (op, part)) in tree.parts.iter().enumerate() {
            if i > 0 {
                sql.push_str(&format!(" {} ", set_op_keyword(op)));
            }
            sql.push_str(&self.render_union_part(part, cfg, &mut binds));
        }

        append_order_by(&mut sql, &tree.order_bys, cfg, " ");
        append_limit_offset_flat(&mut sql, tree.limit, tree.offset);
        (sql, binds)
    }
}

// ── PipeSqlRenderer ──

pub struct PipeSqlRenderer;

impl PipeSqlRenderer {
    fn render_core(&self, tree: &SelectTree, cfg: &RenderConfig, binds: &mut Vec<Value>) -> String {
        let mut parts = Vec::new();

        parts.push(render_from(&tree.from, cfg));

        if let Some(where_sql) = render_wheres(&tree.wheres, cfg, binds) {
            parts.push(format!("WHERE {}", where_sql));
        }

        match &tree.select {
            SelectClause::Columns(cols) => {
                parts.push(render_select_columns(cols, cfg));
            }
            SelectClause::Aggregate { group_bys, exprs } => {
                let agg_exprs: Vec<String> = exprs
                    .iter()
                    .map(|e| render_aggregate_expr(e, cfg))
                    .collect();
                let mut clause = format!("AGGREGATE {}", agg_exprs.join(", "));
                if !group_bys.is_empty() {
                    let cols: Vec<String> = group_bys.iter().map(|c| (cfg.qi)(c)).collect();
                    clause.push_str(&format!(" GROUP BY {}", cols.join(", ")));
                }
                parts.push(clause);
            }
        }

        parts.join(" |> ")
    }

    fn render_union_part(
        &self,
        tree: &SelectTree,
        cfg: &RenderConfig,
        binds: &mut Vec<Value>,
    ) -> String {
        let mut sql = self.render_core(tree, cfg, binds);
        let has_extra = !tree.order_bys.is_empty() || tree.limit.is_some() || tree.offset.is_some();

        if has_extra {
            append_order_by(&mut sql, &tree.order_bys, cfg, " |> ");
            append_limit_offset_pipe(&mut sql, tree.limit, tree.offset);
            sql = format!("({})", sql);
        }

        sql
    }
}

impl Renderer for PipeSqlRenderer {
    fn render_select(&self, tree: &SelectTree, cfg: &RenderConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut sql = self.render_core(tree, cfg, &mut binds);
        append_order_by(&mut sql, &tree.order_bys, cfg, " |> ");
        append_limit_offset_pipe(&mut sql, tree.limit, tree.offset);
        (sql, binds)
    }

    fn render_union(&self, tree: &UnionTree, cfg: &RenderConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut sql = String::new();

        for (i, (op, part)) in tree.parts.iter().enumerate() {
            if i > 0 {
                sql.push_str(&format!(" |> {} ", set_op_keyword(op)));
            }
            sql.push_str(&self.render_union_part(part, cfg, &mut binds));
        }

        append_order_by(&mut sql, &tree.order_bys, cfg, " |> ");
        append_limit_offset_pipe(&mut sql, tree.limit, tree.offset);
        (sql, binds)
    }
}

// ── Shared rendering helpers ──

fn set_op_keyword(op: &crate::SetOp) -> &'static str {
    match op {
        crate::SetOp::Union => "UNION",
        crate::SetOp::UnionAll => "UNION ALL",
    }
}

fn append_order_by(sql: &mut String, order_bys: &[OrderByClause], cfg: &RenderConfig, sep: &str) {
    if let Some(clause) = render_order_by(order_bys, cfg) {
        sql.push_str(sep);
        sql.push_str(&clause);
    }
}

/// Append LIMIT/OFFSET as separate space-separated clauses (standard SQL style).
fn append_limit_offset_flat(sql: &mut String, limit: Option<u64>, offset: Option<u64>) {
    let (l, o) = render_limit_offset(limit, offset);
    if let Some(l) = l {
        sql.push_str(&format!(" {}", l));
    }
    if let Some(o) = o {
        sql.push_str(&format!(" {}", o));
    }
}

/// Append LIMIT/OFFSET as a single pipe stage (pipe SQL style).
fn append_limit_offset_pipe(sql: &mut String, limit: Option<u64>, offset: Option<u64>) {
    let (l, o) = render_limit_offset(limit, offset);
    let mut lo_parts = Vec::new();
    if let Some(l) = l {
        lo_parts.push(l);
    }
    if let Some(o) = o {
        lo_parts.push(o);
    }
    if !lo_parts.is_empty() {
        sql.push_str(&format!(" |> {}", lo_parts.join(" ")));
    }
}

fn render_where_clause(
    clause: &WhereClause,
    is_top_level: bool,
    cfg: &RenderConfig,
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

fn render_wheres(
    wheres: &[WhereEntry],
    cfg: &RenderConfig,
    binds: &mut Vec<Value>,
) -> Option<String> {
    if wheres.is_empty() {
        return None;
    }

    let single = wheres.len() == 1;
    let mut sql = String::new();

    for (i, entry) in wheres.iter().enumerate() {
        let (connector, clause) = match entry {
            WhereEntry::And(c) => ("AND", c),
            WhereEntry::Or(c) => ("OR", c),
        };

        if i > 0 {
            sql.push_str(&format!(" {} ", connector));
        }

        sql.push_str(&render_where_clause(clause, single, cfg, binds));
    }

    Some(sql)
}

fn render_aggregate_expr(expr: &AggregateExpr, cfg: &RenderConfig) -> String {
    let func_str = match &expr.expr {
        AggregateFunc::CountAll => "COUNT(*)".to_string(),
        AggregateFunc::Count(col) => format!("COUNT({})", (cfg.qi)(col)),
        AggregateFunc::Sum(col) => format!("SUM({})", (cfg.qi)(col)),
        AggregateFunc::Avg(col) => format!("AVG({})", (cfg.qi)(col)),
        AggregateFunc::Min(col) => format!("MIN({})", (cfg.qi)(col)),
        AggregateFunc::Max(col) => format!("MAX({})", (cfg.qi)(col)),
        AggregateFunc::Expr(raw) => raw.clone(),
    };
    match &expr.alias {
        Some(alias) => format!("{} AS {}", func_str, (cfg.qi)(alias)),
        None => func_str,
    }
}

fn render_from(from: &FromClause, cfg: &RenderConfig) -> String {
    let mut s = format!("FROM {}", (cfg.qi)(&from.table));
    for suffix in &from.table_suffix {
        s.push(' ');
        s.push_str(suffix);
    }
    s
}

fn render_order_by(order_bys: &[OrderByClause], cfg: &RenderConfig) -> Option<String> {
    if order_bys.is_empty() {
        return None;
    }
    let clauses: Vec<String> = order_bys
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

fn render_limit_offset(
    limit: Option<u64>,
    offset: Option<u64>,
) -> (Option<String>, Option<String>) {
    (
        limit.map(|n| format!("LIMIT {}", n)),
        offset.map(|n| format!("OFFSET {}", n)),
    )
}

fn render_select_columns(cols: &[String], cfg: &RenderConfig) -> String {
    if cols.is_empty() {
        "SELECT *".to_string()
    } else {
        let quoted: Vec<String> = cols.iter().map(|c| (cfg.qi)(c)).collect();
        format!("SELECT {}", quoted.join(", "))
    }
}
