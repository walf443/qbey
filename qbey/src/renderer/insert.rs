use super::RenderConfig;
use crate::tree::{InsertToken, InsertTree};

/// Render an INSERT statement from an `InsertTree`.
pub fn render_insert<V: Clone>(tree: &InsertTree<V>, cfg: &RenderConfig) -> (String, Vec<V>) {
    let mut binds: Vec<V> = Vec::new();
    let mut parts = Vec::new();

    // Extract InsertInto metadata first (required by Values/SelectSource).
    let (table, columns, col_exprs) = extract_insert_into(&tree.tokens);

    for token in &tree.tokens {
        match token {
            InsertToken::InsertInto { .. } => {
                // Already extracted above; nothing to emit here.
            }
            InsertToken::Values(rows) => {
                let mut quoted_cols: Vec<String> = columns.iter().map(|c| (cfg.qi)(c)).collect();
                for (col, _) in col_exprs {
                    quoted_cols.push((cfg.qi)(col));
                }
                let mut sql = format!(
                    "INSERT INTO {} ({}) VALUES ",
                    (cfg.qi)(table),
                    quoted_cols.join(", ")
                );

                for (i, row) in rows.iter().enumerate() {
                    if i > 0 {
                        sql.push_str(", ");
                    }
                    sql.push('(');
                    for (j, val) in row.iter().enumerate() {
                        if j > 0 {
                            sql.push_str(", ");
                        }
                        binds.push(val.clone());
                        sql.push_str(&(cfg.ph)(binds.len()));
                    }
                    for (k, (_, expr)) in col_exprs.iter().enumerate() {
                        if !row.is_empty() || k > 0 {
                            sql.push_str(", ");
                        }
                        sql.push_str(expr);
                    }
                    sql.push(')');
                }

                parts.push(sql);
            }
            InsertToken::SelectSource(sub) => {
                let sub_sql = super::render_subquery_sql(sub, cfg, &mut binds);
                parts.push(format!("INSERT INTO {} {}", (cfg.qi)(table), sub_sql));
            }
            InsertToken::Raw(s) => {
                parts.push(s.clone());
            }
            InsertToken::KeywordAssignments { keyword, sets } => {
                let mut items = Vec::new();
                for clause in sets {
                    match clause {
                        crate::SetClause::Value(col, val) => {
                            binds.push(val.clone());
                            items.push(format!("{} = {}", (cfg.qi)(col), (cfg.ph)(binds.len())));
                        }
                        crate::SetClause::Expr(expr) => {
                            items.push(expr.as_str().to_string());
                        }
                    }
                }
                parts.push(format!("{} {}", keyword, items.join(", ")));
            }
        }
    }

    (parts.join(" "), binds)
}

/// Extract table, columns, and col_exprs from the first `InsertInto` token.
///
/// # Panics
///
/// Panics if no `InsertInto` token is found.
fn extract_insert_into<V: Clone>(
    tokens: &[InsertToken<V>],
) -> (&str, &[String], &[(String, String)]) {
    for token in tokens {
        if let InsertToken::InsertInto {
            table,
            columns,
            col_exprs,
        } = token
        {
            return (table, columns, col_exprs);
        }
    }
    unreachable!("InsertTree must contain an InsertInto token")
}
