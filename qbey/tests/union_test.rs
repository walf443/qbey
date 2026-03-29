use std::borrow::Cow;

use qbey::*;

#[test]
fn test_union_all_to_sql() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    let uq = q1.union_all(&q2);

    let (sql, binds) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ?"
    );
    assert_eq!(
        binds,
        vec![
            Value::String("eng".to_string()),
            Value::String("sales".to_string())
        ]
    );
}

#[test]
fn test_union_to_sql() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let uq = q1.union(&q2);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" UNION SELECT \"dept\" FROM \"contractor\""
    );
}

#[test]
fn test_union_all_with_order_by_and_limit() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    let mut uq = q1.union_all(&q2);
    uq.order_by(col("name").asc());
    uq.limit(10);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? ORDER BY \"name\" ASC LIMIT 10"
    );
}

#[test]
fn test_union_query_merge() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    let mut q3 = qbey("contractor");
    q3.and_where(("dept", "eng"));
    q3.select(&["id", "name"]);

    let mut q4 = qbey("contractor");
    q4.and_where(("dept", "sales"));
    q4.select(&["id", "name"]);

    let mut uq1 = q1.union_all(&q2);
    let uq2 = q3.union_all(&q4);
    uq1.add_union_all(&uq2);

    let (sql, _) = uq1.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"contractor\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"contractor\" WHERE \"dept\" = ?"
    );
}

#[test]
fn test_union_with_query_order_by_and_limit() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);
    q1.order_by(col("name").asc());
    q1.limit(5);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);
    q2.order_by(col("name").desc());
    q2.limit(3);

    let mut uq = q1.union_all(&q2);
    uq.order_by(col("id").asc());
    uq.limit(10);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "(SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? ORDER BY \"name\" ASC LIMIT 5) UNION ALL (SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? ORDER BY \"name\" DESC LIMIT 3) ORDER BY \"id\" ASC LIMIT 10"
    );
}

#[test]
fn test_union_with_one_query_having_order_by() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);
    q2.order_by(col("name").asc());
    q2.limit(5);

    let uq = q1.union_all(&q2);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL (SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? ORDER BY \"name\" ASC LIMIT 5)"
    );
}

#[test]
fn test_intersect_to_sql() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let uq = q1.intersect(&q2);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" INTERSECT SELECT \"dept\" FROM \"contractor\""
    );
}

#[test]
fn test_except_to_sql() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let uq = q1.except(&q2);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" EXCEPT SELECT \"dept\" FROM \"contractor\""
    );
}

#[test]
fn test_intersect_all_to_sql() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let uq = q1.intersect_all(&q2);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" INTERSECT ALL SELECT \"dept\" FROM \"contractor\""
    );
}

#[test]
fn test_except_all_to_sql() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let uq = q1.except_all(&q2);

    let (sql, _) = uq.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" EXCEPT ALL SELECT \"dept\" FROM \"contractor\""
    );
}

#[test]
fn test_query_union_with_compound_query() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    let mut q3 = qbey("contractor");
    q3.and_where(("dept", "eng"));
    q3.select(&["id", "name"]);

    let uq = q2.union_all(&q3);
    let result = q1.union_all(&uq);

    let (sql, binds) = result.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"contractor\" WHERE \"dept\" = ?"
    );
    assert_eq!(binds.len(), 3);
}

#[test]
fn test_add_union_on_non_compound_query() {
    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    // Calling add_union_all directly on a non-compound query converts it
    q1.add_union_all(&q2);

    let (sql, binds) = q1.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ? UNION ALL SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = ?"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn test_intersect_with_order_by_and_limit() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let mut q = q1.intersect(&q2);
    q.order_by(col("dept").asc());
    q.limit(5);

    let (sql, _) = q.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" INTERSECT SELECT \"dept\" FROM \"contractor\" ORDER BY \"dept\" ASC LIMIT 5"
    );
}

#[test]
fn test_except_with_order_by_and_limit() {
    let mut q1 = qbey("employee");
    q1.select(&["dept"]);

    let mut q2 = qbey("contractor");
    q2.select(&["dept"]);

    let mut q = q1.except(&q2);
    q.order_by(col("dept").desc());
    q.limit(3);
    q.offset(1);

    let (sql, _) = q.to_sql();
    assert_eq!(
        sql,
        "SELECT \"dept\" FROM \"employee\" EXCEPT SELECT \"dept\" FROM \"contractor\" ORDER BY \"dept\" DESC LIMIT 3 OFFSET 1"
    );
}

#[test]
fn test_compound_query_with_dialect() {
    use qbey::Dialect;

    struct Postgres;
    impl Dialect for Postgres {
        fn placeholder(&self, index: usize) -> Cow<'static, str> {
            Cow::Owned(format!("${}", index))
        }
    }

    let mut q1 = qbey("employee");
    q1.and_where(("dept", "eng"));
    q1.select(&["id", "name"]);

    let mut q2 = qbey("employee");
    q2.and_where(("dept", "sales"));
    q2.select(&["id", "name"]);

    let mut uq = q1.union_all(&q2);
    uq.order_by(col("id").asc());
    uq.limit(10);

    let (sql, binds) = uq.to_sql_with(&Postgres);
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = $1 UNION ALL SELECT \"id\", \"name\" FROM \"employee\" WHERE \"dept\" = $2 ORDER BY \"id\" ASC LIMIT 10"
    );
    assert_eq!(binds.len(), 2);
}
