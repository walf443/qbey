use qbey::*;

/// Verify that into_sql produces the same SQL and bind values as to_sql
/// for each query type. This ensures the drain traversal order matches
/// the renderer traversal order.

#[test]
fn test_select_into_sql_matches_to_sql() {
    let mut q = qbey("users");
    q.select(&["id", "name"]);
    q.and_where(col("status").eq("active"));
    q.and_where(col("age").gt(18));
    q.and_where(col("role").included(&["admin", "editor"]));
    q.order_by(col("name").asc());
    q.limit(10);

    let (sql_ref, binds_ref) = q.to_sql();
    let (sql_own, binds_own) = q.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_select_with_subquery_into_sql_matches_to_sql() {
    let mut sub = qbey("orders");
    sub.select(&["user_id"]);
    sub.and_where(col("total").gt(100));

    let mut q = qbey("users");
    q.and_where(col("id").included(sub));
    q.and_where(col("active").eq(true));

    let (sql_ref, binds_ref) = q.to_sql();
    let (sql_own, binds_own) = q.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_select_with_like_and_between_into_sql_matches_to_sql() {
    let mut q = qbey("users");
    q.and_where(col("name").like(LikeExpression::contains("test")));
    q.and_where(col("age").between(20, 60));
    q.and_where(col("score").not_between(0, 10));

    let (sql_ref, binds_ref) = q.to_sql();
    let (sql_own, binds_own) = q.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_select_with_exists_into_sql_matches_to_sql() {
    let mut sub = qbey("orders");
    sub.select(&["id"]);
    sub.and_where(("user_id", 1));

    let mut q = qbey("users");
    q.and_where(exists(sub));

    let (sql_ref, binds_ref) = q.to_sql();
    let (sql_own, binds_own) = q.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_select_with_raw_sql_binds_into_sql_matches_to_sql() {
    let mut q = qbey("users");
    q.add_select_expr(
        RawSql::new("COALESCE({}, {})").binds(&["default", "fallback"]),
        Some("val"),
    );
    q.and_where(col("id").eq(1));
    q.order_by_expr(RawSql::new("FIELD({})").binds(&[42]));

    let (sql_ref, binds_ref) = q.to_sql();
    let (sql_own, binds_own) = q.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_union_into_sql_matches_to_sql() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    let mut uq = q1.union_all(&q2);
    uq.order_by(col("name").asc());
    uq.limit(10);

    let (sql_ref, binds_ref) = uq.to_sql();
    let (sql_own, binds_own) = uq.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_insert_into_sql_matches_to_sql() {
    let mut ins = qbey("events").into_insert();
    for i in 0..10 {
        ins.add_value(&[
            ("user_id", Value::Int(i)),
            ("name", Value::String(format!("user_{}", i))),
        ]);
    }

    let (sql_ref, binds_ref) = ins.to_sql();
    let (sql_own, binds_own) = ins.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_update_into_sql_matches_to_sql() {
    let mut u = qbey("employee").into_update();
    u.set(col("name"), "Alice");
    u.set(col("age"), 30);
    u.set_expr(RawSql::new(r#""score" = "score" + {}"#).binds(&[10]));
    let u = u.and_where(col("id").eq(1));

    let (sql_ref, binds_ref) = u.to_sql();
    let (sql_own, binds_own) = u.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_delete_into_sql_matches_to_sql() {
    let d = qbey("employee").into_delete().and_where(col("id").eq(1));

    let (sql_ref, binds_ref) = d.to_sql();
    let (sql_own, binds_own) = d.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}

#[test]
fn test_select_with_cte_into_sql_matches_to_sql() {
    let mut sub = qbey("orders");
    sub.select(&["user_id"]);
    sub.and_where(col("total").gt(1000));

    let mut q = qbey("users");
    q.with_cte("high_value", &[], sub);
    q.and_where(col("active").eq(true));

    let (sql_ref, binds_ref) = q.to_sql();
    let (sql_own, binds_own) = q.into_sql();

    assert_eq!(sql_ref, sql_own);
    assert_eq!(binds_ref, binds_own);
}
