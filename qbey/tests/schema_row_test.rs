//! `row = Name` in `qbey_schema!` — a generated INSERT row builder with one
//! setter per column, type-checked like `set()` / `value()`.

use qbey::prelude::*;
use qbey::{ToInsertRow, Value, qbey, qbey_schema, qbey_with};

#[derive(Debug, Clone, PartialEq)]
struct UserId(i64);

impl From<UserId> for Value {
    fn from(id: UserId) -> Self {
        Value::Int(id.0)
    }
}

qbey_schema!(Comments, "comments", [
    id: i64,
    user_id: UserId,
    body: String,
    likes: i64,
    is_new = "new",
    r#type,
], row = CommentsRow);

#[test]
fn row_builder_sets_typed_and_untyped_columns() {
    let t = Comments::new();
    let mut ins = qbey(&t).into_insert();
    let mut row = t.row();
    row.user_id(UserId(7))
        .body("hello")
        .likes(3i64)
        .is_new(true)
        .r#type("plain");
    ins.add_value(&row);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("user_id", "body", "likes", "new", "type") VALUES (?, ?, ?, ?, ?)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Int(7),
            Value::String("hello".to_string()),
            Value::Int(3),
            Value::Bool(true),
            Value::String("plain".to_string()),
        ]
    );
}

#[test]
fn row_builder_accepts_references() {
    let user_id = UserId(1);
    let body = String::from("by ref");
    let t = Comments::new();
    let mut row = t.row();
    row.user_id(&user_id).body(&body);
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&row);
    let (_sql, binds) = ins.to_sql();
    assert_eq!(
        binds,
        vec![Value::Int(1), Value::String("by ref".to_string())]
    );
    assert_eq!(user_id, UserId(1));
}

#[test]
fn row_builder_can_be_passed_inline() {
    let t = Comments::new();
    let mut ins = qbey(&t).into_insert();
    ins.add_value(t.row().user_id(UserId(1)).likes(0i64));
    ins.add_value(t.row().likes(5i64).user_id(UserId(2)));
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("user_id", "likes") VALUES (?, ?), (?, ?)"#
    );
    assert_eq!(
        binds,
        vec![Value::Int(1), Value::Int(0), Value::Int(2), Value::Int(5)]
    );
}

/// `V` is normally inferred at `add_value()`; when the row is inspected
/// directly, the type parameter's default (`Value`) can be named instead.
#[test]
fn row_builder_conditional_columns() {
    let t = Comments::new();
    let ids = [Some(10i64), None];
    let mut rows = Vec::new();
    for id in ids {
        let mut row: CommentsRow = t.row();
        row.user_id(UserId(1));
        if let Some(id) = id {
            row.id(id);
        }
        rows.push(row);
    }
    assert_eq!(rows[0].to_insert_row().len(), 2);
    assert_eq!(rows[1].to_insert_row().len(), 1);
}

#[test]
fn row_builder_via_add_values() {
    let t = Comments::new();
    let rows: Vec<CommentsRow<Value>> = (1..=2)
        .map(|i| {
            let mut row = t.row();
            row.user_id(UserId(i)).likes(i * 10);
            row
        })
        .collect();
    let mut ins = qbey(&t).into_insert();
    ins.add_values(&rows);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("user_id", "likes") VALUES (?, ?), (?, ?)"#
    );
    assert_eq!(
        binds,
        vec![Value::Int(1), Value::Int(10), Value::Int(2), Value::Int(20)]
    );
}

/// The row type is nameable, so a conversion helper can return it.
fn to_row(user_id: &UserId, body: &str) -> CommentsRow<Value> {
    let t = Comments::new();
    let mut row = t.row();
    row.user_id(user_id).body(body);
    row
}

#[test]
fn row_builder_from_helper_fn() {
    let t = Comments::new();
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&to_row(&UserId(3), "x"));
    let (sql, _binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("user_id", "body") VALUES (?, ?)"#
    );
}

#[derive(Debug, Clone, PartialEq)]
enum MyValue {
    Int(i64),
    Text(String),
}

impl From<UserId> for MyValue {
    fn from(id: UserId) -> Self {
        MyValue::Int(id.0)
    }
}

impl From<String> for MyValue {
    fn from(s: String) -> Self {
        MyValue::Text(s)
    }
}

#[test]
fn row_builder_with_custom_value_type() {
    let t = Comments::new();
    let mut ins = qbey_with::<MyValue>(&t).into_insert();
    let mut row = t.row();
    row.user_id(UserId(4)).body("yo");
    ins.add_value(&row);
    let (_sql, binds) = ins.to_sql();
    assert_eq!(
        binds,
        vec![MyValue::Int(4), MyValue::Text("yo".to_string())]
    );
}

/// Aliasing the schema does not leak into the row: INSERT columns are bare.
#[test]
fn row_builder_ignores_schema_alias() {
    let t = Comments::new().as_("c");
    let mut ins = qbey("comments").into_insert();
    ins.add_value(t.row().likes(1i64));
    let (sql, _binds) = ins.to_sql();
    assert_eq!(sql, r#"INSERT INTO "comments" ("likes") VALUES (?)"#);
}

// A schema without `row = ...` still compiles exactly as before.
qbey_schema!(Plain, "plain", [id, name]);

#[test]
fn schema_without_row_is_unchanged() {
    let p = Plain::new();
    assert_eq!(p.all_columns().len(), 2);
}

/// A default followed by an override must not leave a duplicate column
/// behind; the last value wins and the column keeps its original position.
#[test]
fn row_builder_setter_replaces_earlier_value() {
    let t = Comments::new();
    let mut row: CommentsRow = t.row();
    row.likes(0i64).user_id(UserId(1));
    row.likes(9i64);
    assert_eq!(
        row.clone().into_pairs(),
        vec![
            ("likes".to_string(), Value::Int(9)),
            ("user_id".to_string(), Value::Int(1)),
        ]
    );

    let mut ins = qbey(&t).into_insert();
    ins.add_value(&row);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("likes", "user_id") VALUES (?, ?)"#
    );
    assert_eq!(binds, vec![Value::Int(9), Value::Int(1)]);
}
