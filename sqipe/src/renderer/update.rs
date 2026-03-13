use super::{RenderConfig, render_wheres};
use crate::tree::UpdateTree;

/// Render an UPDATE statement from an `UpdateTree`.
pub fn render_update<V: Clone>(tree: &UpdateTree<V>, cfg: &RenderConfig) -> (String, Vec<V>) {
    let mut binds: Vec<V> = Vec::new();
    let mut parts = Vec::new();

    // UPDATE "table"
    let table = match &tree.table_alias {
        Some(alias) => format!("UPDATE {} AS {}", (cfg.qi)(&tree.table), (cfg.qi)(alias)),
        None => format!("UPDATE {}", (cfg.qi)(&tree.table)),
    };
    parts.push(table);

    // SET "col1" = ?, "col2" = ?
    let set_items: Vec<String> = tree
        .sets
        .iter()
        .map(|(col, val)| {
            binds.push(val.clone());
            let placeholder = (cfg.ph)(binds.len());
            format!("{} = {}", (cfg.qi)(col), placeholder)
        })
        .collect();
    parts.push(format!("SET {}", set_items.join(", ")));

    // WHERE ...
    if let Some(where_sql) = render_wheres(&tree.wheres, cfg, &mut binds) {
        parts.push(format!("WHERE {}", where_sql));
    }

    (parts.join(" "), binds)
}
