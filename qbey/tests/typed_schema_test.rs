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
struct LivestreamId(i64);

impl From<UserId> for Value {
    fn from(id: UserId) -> Self {
        Value::Int(id.0)
    }
}

impl From<LivestreamId> for Value {
    fn from(id: LivestreamId) -> Self {
        Value::Int(id.0)
    }
}

qbey_schema!(Livecomments, "livecomments", [
    id: LivestreamCommentId,
    user_id: UserId,
    livestream_id: LivestreamId,
    comment: String,
    tip: i64,
]);

#[derive(Debug, Clone, PartialEq)]
struct LivestreamCommentId(i64);

impl From<LivestreamCommentId> for Value {
    fn from(id: LivestreamCommentId) -> Self {
        Value::Int(id.0)
    }
}

#[test]
fn typed_col_eq_binds_through_into_value() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().eq(UserId(7)));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."user_id" = ?"#
    );
    assert_eq!(binds, vec![Value::Int(7)]);
}

#[test]
fn typed_col_eq_accepts_reference() {
    let user_id = UserId(7);
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().eq(&user_id));
    let (_sql, binds) = q.to_sql();
    assert_eq!(binds, vec![Value::Int(7)]);
    // `user_id` is still usable — `eq` borrowed it.
    assert_eq!(user_id, UserId(7));
}

#[test]
fn typed_string_col_accepts_str_literal() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.comment().eq("hello"));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."comment" = ?"#
    );
    assert_eq!(binds, vec![Value::String("hello".to_string())]);
}

#[test]
fn typed_col_comparisons() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.tip().gt(100));
    q.and_where(t.tip().lte(1000));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."tip" > ? AND "livecomments"."tip" <= ?"#
    );
    assert_eq!(binds, vec![Value::Int(100), Value::Int(1000)]);
}

#[test]
fn typed_col_included_takes_slice_of_the_column_type() {
    let ids = vec![UserId(1), UserId(2), UserId(3)];
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().included(&ids));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."user_id" IN (?, ?, ?)"#
    );
    assert_eq!(binds, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
}

#[test]
fn typed_col_not_included() {
    let ids = vec![UserId(1)];
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.user_id().not_included(&ids));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."user_id" NOT IN (?)"#
    );
}

#[test]
fn typed_col_between_and_range() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.and_where(t.tip().between(10, 20));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."tip" BETWEEN ? AND ?"#
    );
    assert_eq!(binds, vec![Value::Int(10), Value::Int(20)]);

    let mut q2 = qbey(&t);
    q2.and_where(t.tip().in_range(10..20));
    let (sql2, _) = q2.to_sql();
    assert_eq!(
        sql2,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."tip" >= ? AND "livecomments"."tip" < ?"#
    );
}

qbey_schema!(Livestreams, "livestreams", [
    id: LivestreamId,
    owner_id: UserId,
    title: String,
]);

#[test]
fn typed_join_on_matching_id_types() {
    let c = Livecomments::new();
    let s = Livestreams::new();
    let mut q = qbey(&c);
    q.join(&s, c.livestream_id().eq(s.id()));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" INNER JOIN "livestreams" ON "livecomments"."livestream_id" = "livestreams"."id""#
    );
}

#[test]
fn typed_col_works_in_select_and_order_by() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.select(&t.all_columns());
    q.order_by(t.tip().desc());
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "livecomments"."id", "livecomments"."user_id", "livecomments"."livestream_id", "livecomments"."comment", "livecomments"."tip" FROM "livecomments" ORDER BY "livecomments"."tip" DESC"#
    );
}

#[test]
fn typed_col_alias_keeps_the_type() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    q.add_select(t.tip().as_("amount"));
    q.and_where(t.tip().eq(5i64));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "livecomments"."tip" AS "amount" FROM "livecomments" WHERE "livecomments"."tip" = ?"#
    );
}

/// `select(&[...])` needs a homogeneous slice, so columns of differing types
/// must be unwrapped with `into_col()` first.
#[test]
fn mixed_type_select_via_into_col() {
    let t = Livecomments::new();
    let cols: Vec<Col> = vec![t.id().into_col(), t.comment().into_col()];
    let mut q = qbey(&t);
    q.select(&cols);
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT "livecomments"."id", "livecomments"."comment" FROM "livecomments""#
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
    let t = Livecomments::new();
    let mut q = qbey_with::<MyValue>("livecomments");
    q.and_where(t.user_id().eq(UserId(9)));
    q.and_where(t.comment().eq("hi".to_string()));
    let (sql, binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "livecomments"."user_id" = ? AND "livecomments"."comment" = ?"#
    );
    assert_eq!(
        binds,
        vec![MyValue::Int(9), MyValue::Text("hi".to_string())]
    );
}

#[test]
fn typed_col_interoperates_with_plain_col() {
    let t = Livecomments::new();
    let mut q = qbey(&t);
    // A plain `col()` comparison still works alongside typed ones.
    q.and_where(col("deleted_at").eq("x"));
    q.and_where(t.user_id().eq(UserId(1)));
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" WHERE "deleted_at" = ? AND "livecomments"."user_id" = ?"#
    );
}

#[test]
fn typed_col_order_by_asc_returns_order_by_clause() {
    let t = Livecomments::new();
    let clause: OrderByClause = t.tip().asc();
    let mut q = qbey(&t);
    q.order_by(clause);
    let (sql, _binds) = q.to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "livecomments" ORDER BY "livecomments"."tip" ASC"#
    );
}

// ── UPDATE ... SET ──

#[test]
fn typed_update_set_and_where() {
    let t = Livecomments::new();
    let mut u = qbey(&t).into_update();
    u.set(t.tip(), 100i64);
    u.set(t.comment(), "edited");
    let u = u.and_where(t.user_id().eq(UserId(7)));
    let (sql, binds) = u.to_sql();
    // SET uses the bare column name even though the schema column is qualified.
    assert_eq!(
        sql,
        r#"UPDATE "livecomments" SET "tip" = ?, "comment" = ? WHERE "livecomments"."user_id" = ?"#
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
    let owner = UserId(3);
    let s = Livestreams::new();
    let mut u = qbey(&s).into_update();
    u.set(s.owner_id(), &owner);
    let u = u.and_where(s.id().eq(LivestreamId(1)));
    let (_sql, binds) = u.to_sql();
    assert_eq!(binds, vec![Value::Int(3), Value::Int(1)]);
    assert_eq!(owner, UserId(3));
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
    let t = Livecomments::new();
    let mut u = qbey_with::<MyValue>("livecomments").into_update();
    u.set(t.comment(), "hi");
    let u = u.and_where(t.user_id().eq(UserId(9)));
    let (sql, binds) = u.to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "livecomments" SET "comment" = ? WHERE "livecomments"."user_id" = ?"#
    );
    assert_eq!(
        binds,
        vec![MyValue::Text("hi".to_string()), MyValue::Int(9)]
    );
}

// ── INSERT ──

#[test]
fn typed_insert_row_from_value_pairs() {
    let t = Livecomments::new();
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&[
        t.user_id().value(UserId(1)),
        t.livestream_id().value(LivestreamId(2)),
        t.comment().value("hi"),
        t.tip().value(100i64),
    ]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "livecomments" ("user_id", "livestream_id", "comment", "tip") VALUES (?, ?, ?, ?)"#
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
    let t = Livecomments::new();
    let user_id = UserId(1);
    let row = [t.user_id().value(&user_id), t.comment().value("hi")];
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&row);
    ins.add_value(&[t.comment().value("second"), t.user_id().value(UserId(2))]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "livecomments" ("user_id", "comment") VALUES (?, ?), (?, ?)"#
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
    let t = Livecomments::new();
    let rows: Vec<Vec<(String, Value)>> = (1..=2)
        .map(|i| vec![t.user_id().value(UserId(i)), t.tip().value(i * 10)])
        .collect();
    let mut ins = qbey(&t).into_insert();
    ins.add_values(&rows);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "livecomments" ("user_id", "tip") VALUES (?, ?), (?, ?)"#
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
    let t = Livecomments::new();
    let mut ins = qbey_with::<MyValue>("livecomments").into_insert();
    ins.add_value(&[t.user_id().value(UserId(4)), t.comment().value("yo")]);
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
    let t = Livecomments::new();
    let mut ins = qbey(&t).into_insert();
    ins.add_value(&[t.id().value(LivestreamCommentId(1)), t.tip().value(5i64)]);
    ins.on_conflict_do_update(&[t.id()], t.tip(), 6i64);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "livecomments" ("id", "tip") VALUES (?, ?) ON CONFLICT ("id") DO UPDATE SET "tip" = ?"#
    );
    assert_eq!(binds, vec![Value::Int(1), Value::Int(5), Value::Int(6)]);
}

/// A bare `&str` column name is still accepted by `set()`, as it is by
/// `on_conflict_do_update()`; the name is quoted, never parameterized.
#[test]
fn set_accepts_str_column_name() {
    let mut u = qbey("livecomments").into_update();
    u.set("tip", 1i64);
    let u = u.allow_without_where();
    let (sql, binds) = u.to_sql();
    assert_eq!(sql, r#"UPDATE "livecomments" SET "tip" = ?"#);
    assert_eq!(binds, vec![Value::Int(1)]);
}
