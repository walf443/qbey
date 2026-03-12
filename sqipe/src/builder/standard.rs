use crate::builder::{
    build_aggregate_select, build_group_by_clause, build_limit_offset, build_order_by_clause,
    build_select_clause, build_where, SqlBuilder, SqlConfig,
};
use crate::{Query, SetOp, UnionQuery, Value};

pub(crate) struct StandardSqlBuilder;

impl SqlBuilder for StandardSqlBuilder {
    fn build_core(query: &Query, cfg: &SqlConfig, binds: &mut Vec<Value>) -> String {
        let mut parts = Vec::new();

        if !query.aggregates.is_empty() {
            parts.push(build_aggregate_select(
                &query.group_bys,
                &query.aggregates,
                cfg,
            ));
        } else {
            parts.push(build_select_clause(&query.selects, cfg));
        }

        parts.push(format!("FROM {}", (cfg.qi)(&query.table)));

        if let Some(where_sql) = build_where(&query.wheres, cfg, binds) {
            parts.push(format!("WHERE {}", where_sql));
        }

        if let Some(group_by) = build_group_by_clause(&query.group_bys, cfg) {
            parts.push(group_by);
        }

        parts.join(" ")
    }

    fn build_full(query: &Query, cfg: &SqlConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut sql = Self::build_core(query, cfg, &mut binds);

        if let Some(order_by) = build_order_by_clause(&query.order_bys, cfg) {
            sql.push_str(&format!(" {}", order_by));
        }

        let (limit, offset) = build_limit_offset(query.limit_val, query.offset_val);
        if let Some(l) = limit {
            sql.push_str(&format!(" {}", l));
        }
        if let Some(o) = offset {
            sql.push_str(&format!(" {}", o));
        }

        (sql, binds)
    }

    fn build_union_part(query: &Query, cfg: &SqlConfig, binds: &mut Vec<Value>) -> String {
        let mut sql = Self::build_core(query, cfg, binds);
        let has_extra = !query.order_bys.is_empty()
            || query.limit_val.is_some()
            || query.offset_val.is_some();

        if has_extra {
            if let Some(order_by) = build_order_by_clause(&query.order_bys, cfg) {
                sql.push_str(&format!(" {}", order_by));
            }
            let (limit, offset) = build_limit_offset(query.limit_val, query.offset_val);
            if let Some(l) = limit {
                sql.push_str(&format!(" {}", l));
            }
            if let Some(o) = offset {
                sql.push_str(&format!(" {}", o));
            }
            sql = format!("({})", sql);
        }

        sql
    }

    fn build_union(union: &UnionQuery, cfg: &SqlConfig) -> (String, Vec<Value>) {
        let mut binds = Vec::new();
        let mut sql = String::new();

        for (i, (op, query)) in union.parts.iter().enumerate() {
            if i > 0 {
                let keyword = match op {
                    SetOp::Union => "UNION",
                    SetOp::UnionAll => "UNION ALL",
                };
                sql.push_str(&format!(" {} ", keyword));
            }
            sql.push_str(&Self::build_union_part(query, cfg, &mut binds));
        }

        if let Some(order_by) = build_order_by_clause(&union.order_bys, cfg) {
            sql.push_str(&format!(" {}", order_by));
        }

        let (limit, offset) = build_limit_offset(union.limit_val, union.offset_val);
        if let Some(l) = limit {
            sql.push_str(&format!(" {}", l));
        }
        if let Some(o) = offset {
            sql.push_str(&format!(" {}", o));
        }

        (sql, binds)
    }
}
