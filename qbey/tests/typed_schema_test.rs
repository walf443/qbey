//! Typed columns in `qbey_schema!` — `col: Type` binds a Rust type to a column so
//! that mismatched values and mismatched joins are rejected at compile time.
//!
//! Negative cases live as `compile_fail` doctests on `TypedCol`; a runtime test
//! cannot assert that something fails to compile.

use qbey::prelude::*;
use qbey::{Col, OrderByClause, Value, col, qbey, qbey_schema, qbey_with};

/// A newtype ID, standing in for something like a `kubetsu` ID.
#[derive(Debug, Clone, PartialEq)]
struct UserId(i64);

/// A second ID type — distinct from `UserId` even though both wrap `i64`.
#[derive(Debug, Clone, PartialEq)]
struct PostId(i64);

impl From<UserId> for Value {
    fn from(id: UserId) -> Self {
        Value::Int(id.0)
    }
}

impl From<PostId> for Value {
    fn from(id: PostId) -> Self {
        Value::Int(id.0)
    }
}

qbey_schema!(Comments, "comments", [
    id: CommentId,
    user_id: UserId,
    post_id: PostId,
    body: String,
    likes: i64,
]);

#[derive(Debug, Clone, PartialEq)]
struct CommentId(i64);

impl From<CommentId> for Value {
    fn from(id: CommentId) -> Self {
        Value::Int(id.0)
    }
}

#[test]
fn typed_col_eq_binds_through_into_value() {
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().eq(UserId(7)));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."user_id" = ?"#
    );
    assert_eq!(binds, vec![Value::Int(7)]);
}

#[test]
fn typed_col_eq_accepts_reference() {
    let user_id = UserId(7);
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().eq(&user_id));
    let (_sql, binds) = q.to_sql();
    assert_eq!(binds, vec![Value::Int(7)]);
    // `user_id` is still usable — `eq` borrowed it.
    assert_eq!(user_id, UserId(7));
}

#[test]
fn typed_string_col_accepts_str_literal() {
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.body().eq("hello"));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."body" = ?"#
    );
    assert_eq!(binds, vec![Value::String("hello".to_string())]);
}

#[test]
fn typed_col_comparisons() {
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.likes().gt(100));
    q.and_where(t.likes().lte(1000));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."likes" > ? AND "comments"."likes" <= ?"#
    );
    assert_eq!(binds, vec![Value::Int(100), Value::Int(1000)]);
}

#[test]
fn typed_col_included_takes_slice_of_the_column_type() {
    let ids = vec![UserId(1), UserId(2), UserId(3)];
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().included(&ids));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."user_id" IN (?, ?, ?)"#
    );
    assert_eq!(binds, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
}

#[test]
fn typed_col_not_included() {
    let ids = vec![UserId(1)];
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().not_included(&ids));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."user_id" NOT IN (?)"#
    );
}

#[test]
fn typed_col_between_and_range() {
    let t = Comments::new();
    let mut q = qbey(&t);
    q.and_where(t.likes().between(10, 20));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."likes" BETWEEN ? AND ?"#
    );
    assert_eq!(binds, vec![Value::Int(10), Value::Int(20)]);

    let mut q2 = qbey(&t);
    q2.and_where(t.likes().in_range(10..20));
    let (sql2, _) = q2.to_sql();
    assert_eq!(
        sql2,
        r#"SELECT * FROM "comments" WHERE "comments"."likes" >= ? AND "comments"."likes" < ?"#
    );
}

qbey_schema!(Posts, "posts", [
    id: PostId,
    author_id: UserId,
    title: String,
]);

#[test]
fn typed_join_on_matching_id_types() {
    let c = Comments::new();
    let s = Posts::new();
    let mut q = qbey(&c);
    q.join(&s, c.post_id().eq(s.id()));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" INNER JOIN "posts" ON "comments"."post_id" = "posts"."id""#
    );
}

#[test]
fn typed_col_works_in_select_and_order_by() {
    let t = Comments::new();
    let mut q = qbey(&t);
    q.select(&t.all_columns());
    q.order_by(t.likes().desc());
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "comments"."id", "comments"."user_id", "comments"."post_id", "comments"."body", "comments"."likes" FROM "comments" ORDER BY "comments"."likes" DESC"#
    );
}

#[test]
fn typed_col_alias_keeps_the_type() {
    let t = Comments::new();
    let mut q = qbey(&t);
    q.add_select(t.likes().as_("amount"));
    q.and_where(t.likes().eq(5i64));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "comments"."likes" AS "amount" FROM "comments" WHERE "comments"."likes" = ?"#
    );
}

/// `select(&[...])` needs a homogeneous slice, so columns of differing types
/// must be unwrapped with `into_col()` first.
#[test]
fn mixed_type_select_via_into_col() {
    let t = Comments::new();
    let cols: Vec<Col> = vec![t.id().into_col(), t.body().into_col()];
    let mut q = qbey(&t);
    q.select(&cols);
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "comments"."id", "comments"."body" FROM "comments""#
    );
}

// A schema mixing typed, untyped and renamed columns.
qbey_schema!(Features, "features", [
    id: UserId,
    name,
    is_new = "new",
    score: i64 = "score_value",
]);

#[test]
fn mixed_typed_untyped_and_renamed_columns() {
    let f = Features::new();
    let mut q = qbey(&f);
    q.select(&f.all_columns());
    q.and_where(f.id().eq(UserId(1)));
    // Untyped column keeps the loose API.
    q.and_where(f.name().eq("x"));
    q.and_where(f.score().eq(3i64));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "features"."id", "features"."name", "features"."new", "features"."score_value" FROM "features" WHERE "features"."id" = ? AND "features"."name" = ? AND "features"."score_value" = ?"#
    );
}

/// The whole point of the feature: a custom bind-value type (as a driver would
/// use) receives the ID type without it being unwrapped to i64 by hand.
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

impl From<&str> for MyValue {
    fn from(s: &str) -> Self {
        MyValue::Text(s.to_string())
    }
}

#[test]
fn typed_col_with_custom_value_type() {
    let t = Comments::new();
    let mut q = qbey_with::<MyValue>("comments");
    q.and_where(t.user_id().eq(UserId(9)));
    q.and_where(t.body().eq("hi".to_string()));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "comments"."user_id" = ? AND "comments"."body" = ?"#
    );
    assert_eq!(
        binds,
        vec![MyValue::Int(9), MyValue::Text("hi".to_string())]
    );
}

#[test]
fn typed_col_interoperates_with_plain_col() {
    let t = Comments::new();
    let mut q = qbey(&t);
    // A plain `col()` comparison still works alongside typed ones.
    q.and_where(col("deleted_at").eq("x"));
    q.and_where(t.user_id().eq(UserId(1)));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" WHERE "deleted_at" = ? AND "comments"."user_id" = ?"#
    );
}

#[test]
fn typed_col_order_by_asc_returns_order_by_clause() {
    let t = Comments::new();
    let clause: OrderByClause = t.likes().asc();
    let mut q = qbey(&t);
    q.order_by(clause);
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "comments" ORDER BY "comments"."likes" ASC"#
    );
}

// ── UPDATE ... SET ──

#[test]
fn typed_update_set_and_where() {
    let t = Comments::new();
    let mut u = qbey(&t).into_update();
    u.set(t.likes(), 100i64);
    u.set(t.body(), "edited");
    let u = u.and_where(t.user_id().eq(UserId(7)));
    let (sql, binds) = u.to_sql();
    // SET uses the bare column name even though the schema column is qualified.
    assert_eq!(
        sql,
        r#"UPDATE "comments" SET "likes" = ?, "body" = ? WHERE "comments"."user_id" = ?"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Int(100),
            Value::String("edited".to_string()),
            Value::Int(7)
        ]
    );
}

#[test]
fn typed_update_set_accepts_reference() {
    let author = UserId(3);
    let s = Posts::new();
    let mut u = qbey(&s).into_update();
    u.set(s.author_id(), &author);
    let u = u.and_where(s.id().eq(PostId(1)));
    let (_sql, binds) = u.to_sql();
    assert_eq!(binds, vec![Value::Int(3), Value::Int(1)]);
    assert_eq!(author, UserId(3));
}

#[test]
fn untyped_set_still_takes_plain_col() {
    let f = Features::new();
    let mut u = qbey(&f).into_update();
    u.set(f.name(), "x");
    u.set(col("age"), 30);
    u.set(f.score(), 5i64);
    let u = u.allow_without_where();
    let (sql, binds) = u.to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "features" SET "name" = ?, "age" = ?, "score_value" = ?"#
    );
    assert_eq!(
        binds,
        vec![
            Value::String("x".to_string()),
            Value::Int(30),
            Value::Int(5)
        ]
    );
}

#[test]
fn typed_update_set_with_custom_value_type() {
    let t = Comments::new();
    let mut u = qbey_with::<MyValue>("comments").into_update();
    u.set(t.body(), "hi");
    let u = u.and_where(t.user_id().eq(UserId(9)));
    let (sql, binds) = u.to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "comments" SET "body" = ? WHERE "comments"."user_id" = ?"#
    );
    assert_eq!(
        binds,
        vec![MyValue::Text("hi".to_string()), MyValue::Int(9)]
    );
}

// ── INSERT ──

#[test]
fn typed_insert_row_from_value_pairs() {
    let t = Comments::new();
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&[
        t.user_id().value(UserId(1)),
        t.post_id().value(PostId(2)),
        t.body().value("hi"),
        t.likes().value(100i64),
    ]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("user_id", "post_id", "body", "likes") VALUES (?, ?, ?, ?)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Int(1),
            Value::Int(2),
            Value::String("hi".to_string()),
            Value::Int(100),
        ]
    );
}

/// `V` is only pinned down when the row reaches `add_value()`, so building the
/// row in a separate statement must still infer.
#[test]
fn typed_insert_row_built_separately() {
    let t = Comments::new();
    let user_id = UserId(1);
    let row = [t.user_id().value(&user_id), t.body().value("hi")];
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&row);
    ins.add_value(&[t.body().value("second"), t.user_id().value(UserId(2))]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("user_id", "body") VALUES (?, ?), (?, ?)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Int(1),
            Value::String("hi".to_string()),
            Value::Int(2),
            Value::String("second".to_string()),
        ]
    );
}

#[test]
fn typed_insert_rows_from_vec_via_add_values() {
    let t = Comments::new();
    let rows: Vec<Vec<(String, Value)>> = (1..=2)
        .map(|i| vec![t.user_id().value(UserId(i)), t.likes().value(i * 10)])
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

#[test]
fn untyped_col_value_pairs_in_insert() {
    let f = Features::new();
    let mut ins = qbey(&f).into_insert();
    ins.add_value(&[f.name().value("x"), f.score().value(5i64)]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "features" ("name", "score_value") VALUES (?, ?)"#
    );
    assert_eq!(binds, vec![Value::String("x".to_string()), Value::Int(5)]);
}

#[test]
fn typed_insert_with_custom_value_type() {
    let t = Comments::new();
    let mut ins = qbey_with::<MyValue>("comments").into_insert();
    ins.add_value(&[t.user_id().value(UserId(4)), t.body().value("yo")]);
    let (_sql, binds) = ins.to_sql();
    assert_eq!(
        binds,
        vec![MyValue::Int(4), MyValue::Text("yo".to_string())]
    );
}

/// `on_conflict_do_update` goes through `ColumnValue` like `set()`, so the
/// value is checked against the column type. The negative case lives as a
/// `compile_fail` doctest on `TypedCol`.
#[cfg(feature = "conflict")]
#[test]
fn typed_col_in_on_conflict_do_update() {
    let t = Comments::new();
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&[t.id().value(CommentId(1)), t.likes().value(5i64)]);
    ins.on_conflict_do_update(&[t.id()], t.likes(), 6i64);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "comments" ("id", "likes") VALUES (?, ?) ON CONFLICT ("id") DO UPDATE SET "likes" = ?"#
    );
    assert_eq!(binds, vec![Value::Int(1), Value::Int(5), Value::Int(6)]);
}

/// A bare `&str` column name is still accepted by `set()`, as it is by
/// `on_conflict_do_update()`; the name is quoted, never parameterized.
#[test]
fn set_accepts_str_column_name() {
    let mut u = qbey("comments").into_update();
    u.set("likes", 1i64);
    let u = u.allow_without_where();
    let (sql, binds) = u.to_sql();
    assert_eq!(sql, r#"UPDATE "comments" SET "likes" = ?"#);
    assert_eq!(binds, vec![Value::Int(1)]);
}
