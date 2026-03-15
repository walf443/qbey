use super::{
    RenderConfig, Renderer, append_limit_offset_flat, append_lock_clause, append_order_by,
    render_select_core, set_op_keyword,
};
use crate::tree::{SelectTree, SetOperationTree};

pub struct StandardSqlRenderer;

impl StandardSqlRenderer {
    fn render_set_operation_part<V: Clone>(
        &self,
        tree: &SelectTree<V>,
        cfg: &RenderConfig,
        binds: &mut Vec<V>,
    ) -> String {
        let mut sql = render_select_core(tree, cfg, binds);
        append_order_by(&mut sql, &tree.order_bys, cfg, " ");
        append_limit_offset_flat(&mut sql, tree.limit, tree.offset);
        append_lock_clause(&mut sql, tree.lock_for.as_deref());

        let has_extra = !tree.order_bys.is_empty() || tree.limit.is_some() || tree.offset.is_some();

        if has_extra {
            sql = format!("({})", sql);
        }

        sql
    }
}

impl Renderer for StandardSqlRenderer {
    fn render_select<V: Clone>(
        &self,
        tree: &SelectTree<V>,
        cfg: &RenderConfig,
    ) -> (String, Vec<V>) {
        let mut binds = Vec::new();
        let mut sql = render_select_core(tree, cfg, &mut binds);
        append_order_by(&mut sql, &tree.order_bys, cfg, " ");
        append_limit_offset_flat(&mut sql, tree.limit, tree.offset);
        append_lock_clause(&mut sql, tree.lock_for.as_deref());
        (sql, binds)
    }

    fn render_set_operation<V: Clone>(
        &self,
        tree: &SetOperationTree<V>,
        cfg: &RenderConfig,
    ) -> (String, Vec<V>) {
        let mut binds = Vec::new();
        let mut sql = String::new();

        for (i, (op, part)) in tree.parts.iter().enumerate() {
            if i > 0 {
                sql.push_str(&format!(" {} ", set_op_keyword(op)));
            }
            sql.push_str(&self.render_set_operation_part(part, cfg, &mut binds));
        }

        append_order_by(&mut sql, &tree.order_bys, cfg, " ");
        append_limit_offset_flat(&mut sql, tree.limit, tree.offset);
        (sql, binds)
    }
}
