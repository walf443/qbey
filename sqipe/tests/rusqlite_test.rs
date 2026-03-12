#![cfg(feature = "test-rusqlite")]

use rusqlite::{Connection, params_from_iter};
use sqipe::{col, sqipe, table};

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            age INTEGER NOT NULL
        );
        CREATE TABLE orders (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            total REAL NOT NULL,
            status TEXT NOT NULL
        );
        INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35);
        INSERT INTO orders (id, user_id, total, status) VALUES (1, 1, 100.0, 'shipped'), (2, 1, 200.0, 'pending'), (3, 2, 50.0, 'shipped');",
    )
    .unwrap();

    conn
}

/// Convert sqipe::Value to a type rusqlite can bind.
fn to_rusqlite_params(binds: &[sqipe::Value]) -> Vec<Box<dyn rusqlite::types::ToSql>> {
    binds
        .iter()
        .map(|v| -> Box<dyn rusqlite::types::ToSql> {
            match v {
                sqipe::Value::String(s) => Box::new(s.clone()),
                sqipe::Value::Int(n) => Box::new(*n),
                sqipe::Value::Float(f) => Box::new(*f),
                sqipe::Value::Bool(b) => Box::new(*b),
            }
        })
        .collect()
}

#[test]
fn test_basic_select() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.select(&["id", "name"]);
    let (sql, _) = q.to_sql();

    let mut stmt = conn.prepare(&sql).unwrap();
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
}

#[test]
fn test_where_condition() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.and_where(("name", "Alice"));
    q.select(&["id", "name", "age"]);
    let (sql, binds) = q.to_sql();

    let params = to_rusqlite_params(&binds);
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows: Vec<(i64, String, i64)> = stmt
        .query_map(params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1, "Alice".to_string(), 30));
}

#[test]
fn test_order_by_and_limit() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.select(&["id", "name"]);
    q.order_by(col("age").desc());
    q.limit(2);
    let (sql, _) = q.to_sql();

    let mut stmt = conn.prepare(&sql).unwrap();
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(names, vec!["Charlie", "Alice"]);
}

#[test]
fn test_join() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.join("orders", table("users").col("id").eq_col("user_id"));
    q.and_where(table("orders").col("status").eq("shipped"));
    q.select_cols(&table("users").cols(&["id", "name"]));
    q.add_select(table("orders").col("total"));
    let (sql, binds) = q.to_sql();

    let params = to_rusqlite_params(&binds);
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows: Vec<(i64, String, f64)> = stmt
        .query_map(params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 2);
}

#[test]
fn test_join_with_alias() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.as_("u");
    q.join(
        table("orders").as_("o"),
        table("u").col("id").eq_col("user_id"),
    );
    q.and_where(table("o").col("status").eq("shipped"));
    let mut cols = table("u").cols(&["id", "name"]);
    cols.extend(table("o").cols(&["total"]));
    q.select_cols(&cols);
    let (sql, binds) = q.to_sql();

    let params = to_rusqlite_params(&binds);
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows: Vec<(i64, String, f64)> = stmt
        .query_map(params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, "Alice");
}

#[test]
fn test_left_join() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.as_("u");
    q.left_join(
        table("orders").as_("o"),
        table("u").col("id").eq_col("user_id"),
    );
    q.select_cols(&table("u").cols(&["id", "name"]));
    q.add_select(table("o").col("total").as_("order_total"));
    let (sql, _) = q.to_sql();

    let mut stmt = conn.prepare(&sql).unwrap();
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    // Alice=2 orders, Bob=1 order, Charlie=0 orders (NULL → 1 row)
    assert_eq!(rows.len(), 4);
}

#[test]
fn test_between() {
    let conn = setup_db();

    let mut q = sqipe("users");
    q.and_where(col("age").between(25, 30));
    q.select(&["id", "name"]);
    q.order_by(col("name").asc());
    let (sql, binds) = q.to_sql();

    let params = to_rusqlite_params(&binds);
    let mut stmt = conn.prepare(&sql).unwrap();
    let names: Vec<String> = stmt
        .query_map(params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            row.get::<_, String>(1)
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(names, vec!["Alice", "Bob"]);
}

#[test]
fn test_aggregate_count() {
    let conn = setup_db();

    let mut q = sqipe("orders");
    q.aggregate(&[sqipe::aggregate::count_all().as_("cnt")]);
    q.group_by(&["status"]);
    q.select(&["status"]);
    let (sql, _) = q.to_sql();

    let mut stmt = conn.prepare(&sql).unwrap();
    let rows: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 2);
}

#[test]
fn test_union() {
    let conn = setup_db();

    use sqipe::UnionQueryOps;

    let mut q1 = sqipe("users");
    q1.and_where(col("age").gt(30));
    q1.select(&["id", "name"]);

    let mut q2 = sqipe("users");
    q2.and_where(col("age").lt(26));
    q2.select(&["id", "name"]);

    let uq = q1.union(&q2);
    let (sql, binds) = uq.to_sql();

    let params = to_rusqlite_params(&binds);
    let mut stmt = conn.prepare(&sql).unwrap();
    let names: Vec<String> = stmt
        .query_map(params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            row.get::<_, String>(1)
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(names.len(), 2); // Charlie (35), Bob (25)
}
