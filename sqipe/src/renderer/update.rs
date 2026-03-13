use super::{RenderConfig, render_wheres};
use crate::tree::UpdateTree;

/// Render an UPDATE statement from an `UpdateTree`.
///
/// # Panics
///
/// Panics if `tree.sets` is empty, as an UPDATE with no SET clause is invalid SQL.
pub(crate) fn render_update<V: Clone>(
    tree: &UpdateTree<V>,
    cfg: &RenderConfig,
) -> (String, Vec<V>) {
    assert!(
        !tree.sets.is_empty(),
        "UPDATE requires at least one SET clause"
    );

    let mut binds: Vec<V> = Vec::new();
    let mut parts = Vec::new();

    // UPDATE "table" or UPDATE "table" "alias"
    let table = match &tree.table_alias {
        Some(alias) => format!("UPDATE {} {}", (cfg.qi)(&tree.table), (cfg.qi)(alias)),
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
