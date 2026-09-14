use crate::qbey;
use qbey::{ConditionExpr, InsertQueryBuilder, SelectQueryBuilder, col};

#[test]
fn test_insert_single_row() {
    let mut ins = qbey("employee").into_insert();
    ins.add_value(&[("name", "Alice".into()), ("age", 30.into())]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(sql, "INSERT INTO `employee` (`name`, `age`) VALUES (?, ?)");
    assert_eq!(
        binds,
        vec![
            qbey::Value::String("Alice".to_string()),
            qbey::Value::Int(30)
        ]
    );
}

#[test]
fn test_insert_multiple_rows() {
    let mut ins = qbey("employee").into_insert();
    ins.add_value(&[("name", "Alice".into()), ("age", 30.into())]);
    ins.add_value(&[("name", "Bob".into()), ("age", 25.into())]);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `employee` (`name`, `age`) VALUES (?, ?), (?, ?)"
    );
    assert_eq!(
        binds,
        vec![
            qbey::Value::String("Alice".to_string()),
            qbey::Value::Int(30),
            qbey::Value::String("Bob".to_string()),
            qbey::Value::Int(25),
        ]
    );
}

#[test]
fn test_insert_from_select() {
    let mut sub = qbey("old_employee");
    sub.select(&["name", "age"]);
    sub.and_where(col("active").eq(true));

    let mut ins = qbey("employee").into_insert();
    ins.from_select(sub);
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `employee` SELECT `name`, `age` FROM `old_employee` WHERE `active` = ?"
    );
    assert_eq!(binds, vec![qbey::Value::Bool(true)]);
}

#[test]
fn test_insert_on_duplicate_key_update_with_value() {
    let mut ins = qbey("users").into_insert();
    ins.add_value(&[("id", 1.into()), ("name", "Alice".into())]);
    ins.on_duplicate_key_update(col("name"), "Alice");
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `users` (`id`, `name`) VALUES (?, ?) ON DUPLICATE KEY UPDATE `name` = ?"
    );
    assert_eq!(
        binds,
        vec![
            qbey::Value::Int(1),
            qbey::Value::String("Alice".to_string()),
            qbey::Value::String("Alice".to_string()),
        ]
    );
}

#[test]
fn test_insert_on_duplicate_key_update_expr() {
    let mut ins = qbey("users").into_insert();
    ins.add_value(&[("id", 1.into()), ("age", 30.into())]);
    ins.on_duplicate_key_update_expr(qbey::RawSql::new("`age` = `age` + 1"));
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `users` (`id`, `age`) VALUES (?, ?) ON DUPLICATE KEY UPDATE `age` = `age` + 1"
    );
    assert_eq!(binds, vec![qbey::Value::Int(1), qbey::Value::Int(30),]);
}

#[test]
fn test_insert_on_duplicate_key_update_multiple() {
    let mut ins = qbey("users").into_insert();
    ins.add_value(&[
        ("id", 1.into()),
        ("name", "Alice".into()),
        ("age", 30.into()),
    ]);
    ins.on_duplicate_key_update_expr(qbey::RawSql::new("`age` = `age` + 1"));
    ins.on_duplicate_key_update(col("name"), "Alicia");
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `users` (`id`, `name`, `age`) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE `age` = `age` + 1, `name` = ?"
    );
    assert_eq!(
        binds,
        vec![
            qbey::Value::Int(1),
            qbey::Value::String("Alice".to_string()),
            qbey::Value::Int(30),
            qbey::Value::String("Alicia".to_string()),
        ]
    );
}

#[test]
fn test_insert_without_on_duplicate_key_update() {
    let mut ins = qbey("users").into_insert();
    ins.add_value(&[("id", 1.into()), ("name", "Alice".into())]);
    let (sql, _) = ins.to_sql();
    assert_eq!(sql, "INSERT INTO `users` (`id`, `name`) VALUES (?, ?)");
}

#[test]
#[should_panic(expected = "duplicate column")]
fn test_insert_on_duplicate_key_update_duplicate_column_panics() {
    let mut ins = qbey("users").into_insert();
    ins.add_value(&[("id", 1.into()), ("name", "Alice".into())]);
    ins.on_duplicate_key_update(col("name"), "Alice");
    ins.on_duplicate_key_update(col("name"), "Bob");
}

#[test]
fn test_insert_typed_col_values_and_on_duplicate_key_update() {
    use qbey::{Value, qbey_schema};

    #[derive(Debug, Clone)]
    struct UserId(i64);
    impl From<UserId> for Value {
        fn from(id: UserId) -> Self {
            Value::Int(id.0)
        }
    }
    qbey_schema!(Users, "users", [id: UserId, name: String]);

    let t = Users::new();
    let mut ins = qbey("users").into_insert();
    ins.add_value(&[t.id().value(UserId(1)), t.name().value("Alice")]);
    ins.on_duplicate_key_update(t.name(), "Alicia");
    let (sql, binds) = ins.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `users` (`id`, `name`) VALUES (?, ?) ON DUPLICATE KEY UPDATE `name` = ?"
    );
    assert_eq!(
        binds,
        vec![
            Value::Int(1),
            Value::String("Alice".to_string()),
            Value::String("Alicia".to_string()),
        ]
    );
}
