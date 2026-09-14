/// Define a typed schema struct for a database table.
///
/// Generates a struct with methods for each column that return [`Col`](crate::Col)
/// references qualified with the table name, supporting aliasing for self-joins.
///
/// # Example
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::qbey;
///
/// qbey_schema!(Users, "users", [id, name, email]);
///
/// let u = Users::new();
/// let mut q = qbey(&u);
/// q.select(&u.all_columns());
/// q.and_where(u.name().eq("Alice"));
/// let (sql, _binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users" WHERE "users"."name" = ?"#);
/// ```
///
/// # Self-join with alias
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::qbey;
///
/// qbey_schema!(Users, "users", [id, name, manager_id]);
///
/// let u = Users::new();
/// let m = Users::new().as_("managers");
/// let mut q = qbey(&u);
/// q.select(&[u.name(), m.name().as_("manager_name")]);
/// q.left_join(
///     &m,
///     u.manager_id().eq(m.id()),
/// );
/// let (sql, _binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT "users"."name", "managers"."name" AS "manager_name" FROM "users" LEFT JOIN "users" AS "managers" ON "users"."manager_id" = "managers"."id""#);
/// ```
///
/// # Reserved words as column names
///
/// Use Rust raw identifiers for columns whose names are Rust reserved words:
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::qbey;
///
/// qbey_schema!(Events, "events", [id, r#type]);
///
/// let e = Events::new();
/// let mut q = qbey(&e);
/// q.add_select(e.r#type());
/// let (sql, _binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT "events"."type" FROM "events""#);
/// ```
///
/// # Renaming columns
///
/// Use `rust_name = "sql_name"` when you want the Rust method name to differ
/// from the SQL column name. This is useful for columns that conflict with
/// built-in method names (`table`, `table_name`, `as_`, `all_columns`, `new`,
/// and `row` when a row builder is generated):
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::qbey;
///
/// qbey_schema!(Features, "features", [id, name, is_new = "new"]);
///
/// let f = Features::new();
/// let mut q = qbey(&f);
/// q.select(&f.all_columns());
/// q.and_where(f.is_new().eq(true));
/// let (sql, _binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT "features"."id", "features"."name", "features"."new" FROM "features" WHERE "features"."new" = ?"#);
/// ```
///
/// The SQL name must be a string literal:
///
/// ```compile_fail
/// use qbey::qbey_schema;
///
/// qbey_schema!(Bad, "bad", [id, col = 42]);
/// ```
///
/// # Typed columns
///
/// A column declared as `name: Type` produces a
/// [`TypedCol<Type>`](crate::TypedCol) instead of a plain [`Col`](crate::Col).
/// Its comparison methods, `set()` in UPDATE and `value()` for INSERT rows
/// accept only that type, so a newtype ID can be passed straight through
/// instead of being unwrapped at every call site:
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::{qbey, Value};
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// impl From<UserId> for Value {
///     fn from(id: UserId) -> Self { Value::Int(id.0) }
/// }
///
/// qbey_schema!(Comments, "comments", [user_id: UserId, likes: i64]);
///
/// let t = Comments::new();
/// let mut q = qbey(&t);
/// q.and_where(t.user_id().eq(UserId(7)));
/// let (sql, binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT * FROM "comments" WHERE "comments"."user_id" = ?"#);
/// assert_eq!(binds, vec![Value::Int(7)]);
/// ```
///
/// Typed, untyped and renamed columns can be mixed, and `Type` may be combined
/// with a rename as `rust_name: Type = "sql_name"`:
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::{qbey, Value};
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// impl From<UserId> for Value {
///     fn from(id: UserId) -> Self { Value::Int(id.0) }
/// }
///
/// qbey_schema!(Users, "users", [id: UserId, name, score: i64 = "score_value"]);
///
/// let u = Users::new();
/// let mut q = qbey(&u);
/// q.select(&u.all_columns());
/// let (sql, _binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT "users"."id", "users"."name", "users"."score_value" FROM "users""#);
/// ```
///
/// See [`TypedCol`](crate::TypedCol) for what is rejected at compile time.
///
/// # INSERT row builder
///
/// Adding `row = Name` after the column list generates a row-builder struct
/// with one setter per column and a `row()` constructor on the schema. Typed
/// columns accept only their own type (or a reference to it); untyped columns
/// accept anything `Into<V>`. The row implements
/// [`ToInsertRow`](crate::ToInsertRow), so it goes straight into
/// [`add_value()`](crate::InsertQueryBuilder::add_value) /
/// [`add_values()`](crate::InsertQueryBuilder::add_values):
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::prelude::*;
/// use qbey::{qbey, Value};
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// impl From<UserId> for Value {
///     fn from(id: UserId) -> Self { Value::Int(id.0) }
/// }
///
/// qbey_schema!(Comments, "comments", [user_id: UserId, body: String, likes: i64], row = CommentsRow);
///
/// let t = Comments::new();
/// let mut ins = qbey(&t).into_insert();
///
/// let mut row = t.row();
/// row.user_id(UserId(7)).body("hello");
/// if true {
///     row.likes(3i64);
/// }
/// ins.add_value(&row);
///
/// // Or inline, when the row is short.
/// ins.add_value(t.row().user_id(UserId(8)).body("second").likes(0i64));
///
/// let (sql, binds) = ins.to_sql();
/// assert_eq!(sql, r#"INSERT INTO "comments" ("user_id", "body", "likes") VALUES (?, ?, ?), (?, ?, ?)"#);
/// assert_eq!(binds[0], Value::Int(7));
/// ```
///
/// Setting a column twice replaces the earlier value, so a default followed
/// by a conditional override is fine. Note that every row given to one
/// INSERT must have the same set of columns — `add_value()` panics
/// otherwise — so a column set conditionally must be conditional in the same
/// way for all rows of that statement, or the rows split into separate
/// INSERTs. A row on which no setter was called is empty, and `add_value()`
/// panics on it, so guard the call when every column is conditional.
///
/// `V` (the bind type) is inferred at the `add_value()` call; name it as
/// `CommentsRow<Value>` (or just `CommentsRow`, whose default is
/// [`Value`](crate::Value)) when a row is built in a helper function. The
/// row-builder methods share the schema's namespace, so a column named `row`
/// must be renamed (`row_ = "row"`) when `row = Name` is used.
///
/// `row.user_id("foo")` is a compile error, like `set()` and `value()`.
///
/// # Adding custom methods
///
/// The generated struct is a regular Rust struct, so you can add your own
/// methods with a separate `impl` block:
///
/// ```
/// use qbey::qbey_schema;
/// use qbey::Col;
///
/// qbey_schema!(Users, "users", [id, name, email]);
///
/// impl Users {
///     /// Returns columns typically needed for a list view.
///     pub fn list_columns(&self) -> Vec<Col> {
///         vec![self.id(), self.name()]
///     }
/// }
///
/// let u = Users::new();
/// assert_eq!(u.list_columns().len(), 2);
/// ```
#[macro_export]
macro_rules! qbey_schema {
    // Entry point: parse column definitions and forward to internal macro.
    // `{}` / `{RowName}` carries the optional row-builder name through parsing.
    ($struct_name:ident, $table_name:expr, [$($col_def:tt)*] $(,)?) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {}, [] $($col_def)*);
    };
    ($struct_name:ident, $table_name:expr, [$($col_def:tt)*], row = $row_name:ident $(,)?) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$row_name}, [] $($col_def)*);
    };
}

/// Internal macro that parses column definitions one by one.
/// Accumulates parsed columns as `[rust_ident, sql_name]` pairs.
#[doc(hidden)]
#[macro_export]
macro_rules! __qbey_schema_parse {
    // Typed + renamed column: `col: Type = "sql_name"` followed by comma and more
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident : $ty:ty = $sql_name:expr, $($rest:tt)*) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col : $ty, $sql_name]] $($rest)*);
    };
    // Typed + renamed column at end
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident : $ty:ty = $sql_name:expr) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col : $ty, $sql_name]]);
    };
    // Typed column: `col: Type` followed by comma and more
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident : $ty:ty, $($rest:tt)*) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col : $ty]] $($rest)*);
    };
    // Typed column at end
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident : $ty:ty) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col : $ty]]);
    };
    // Renamed column: `col = "sql_name"` followed by comma and more
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident = $sql_name:expr, $($rest:tt)*) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col, $sql_name]] $($rest)*);
    };
    // Renamed column: `col = "sql_name"` at end
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident = $sql_name:expr) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col, $sql_name]]);
    };
    // Plain column followed by comma and more
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident, $($rest:tt)*) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col]] $($rest)*);
    };
    // Plain column at end
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] $col:ident) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)* [$col]]);
    };
    // Trailing comma only
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$($parsed:tt)*] ,) => {
        $crate::__qbey_schema_parse!($struct_name, $table_name, {$($row)?}, [$($parsed)*]);
    };
    // Terminal: all columns parsed, generate the struct and the optional row builder
    ($struct_name:ident, $table_name:expr, {$($row:ident)?}, [$([$($col_spec:tt)*])*]) => {
        $crate::__qbey_schema_emit!($struct_name, $table_name, $([$($col_spec)*]),*);
        $crate::__qbey_schema_row!($struct_name, {$($row)?}, $([$($col_spec)*]),*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __qbey_schema_emit {
    ($struct_name:ident, $table_name:expr, $([$($col_spec:tt)*]),*) => {
        const _: &str = $table_name;

        #[allow(dead_code)]
        pub struct $struct_name {
            alias: Option<&'static str>,
        }

        #[allow(dead_code)]
        impl $struct_name {
            pub const fn new() -> Self {
                $struct_name { alias: None }
            }

            pub fn table_name(&self) -> &'static str {
                $table_name
            }

            /// Returns a `TableRef` suitable for FROM/JOIN clauses.
            /// When aliased, returns `table("name").as_("alias")`.
            pub fn table(&self) -> $crate::TableRef {
                match self.alias {
                    Some(alias) => $crate::table($table_name).as_(alias),
                    None => $crate::table($table_name),
                }
            }

            /// Create an aliased copy of this schema, useful for self-joins.
            ///
            /// Column accessors on the returned instance are qualified with the
            /// alias, and `table()` returns `table("original").as_("alias")` so
            /// it can be passed directly to `join` / `left_join`.
            pub fn as_(&self, alias: &'static str) -> Self {
                $struct_name { alias: Some(alias) }
            }

            $($crate::__qbey_schema_col!($table_name, $($col_spec)*);)*

            pub fn all_columns(&self) -> Vec<$crate::Col> {
                vec![$($crate::__qbey_schema_col_call!(self, $($col_spec)*)),*]
            }
        }

        impl $crate::IntoFromTable for &$struct_name {
            fn into_from_table(self) -> (String, Option<String>) {
                self.table().into_from_table()
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __qbey_schema_col {
    ($table_name:expr, $col:ident : $ty:ty, $sql_name:expr) => {
        pub fn $col(&self) -> $crate::TypedCol<$ty> {
            $crate::TypedCol::new($crate::table(self.alias.unwrap_or($table_name)).col($sql_name))
        }
    };
    ($table_name:expr, $col:ident : $ty:ty) => {
        pub fn $col(&self) -> $crate::TypedCol<$ty> {
            let col_name = stringify!($col).trim_start_matches("r#");
            $crate::TypedCol::new($crate::table(self.alias.unwrap_or($table_name)).col(col_name))
        }
    };
    ($table_name:expr, $col:ident, $sql_name:expr) => {
        pub fn $col(&self) -> $crate::Col {
            $crate::table(self.alias.unwrap_or($table_name)).col($sql_name)
        }
    };
    ($table_name:expr, $col:ident) => {
        pub fn $col(&self) -> $crate::Col {
            let col_name = stringify!($col).trim_start_matches("r#");
            $crate::table(self.alias.unwrap_or($table_name)).col(col_name)
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __qbey_schema_col_call {
    // Matches plain, renamed and typed specs alike; the reflexive `From<Col> for Col`
    // makes this work for untyped columns too, so `all_columns()` stays `Vec<Col>`
    // even for a schema that mixes typed and untyped columns.
    ($self:ident, $col:ident $($rest:tt)*) => {
        $crate::Col::from($self.$col())
    };
}

/// Emits the row builder when `row = Name` was given; expands to nothing otherwise.
#[doc(hidden)]
#[macro_export]
macro_rules! __qbey_schema_row {
    ($struct_name:ident, {}, $([$($col_spec:tt)*]),*) => {};
    ($struct_name:ident, {$row_name:ident}, $([$($col_spec:tt)*]),*) => {
        /// An INSERT row for the table, with one setter per column.
        ///
        /// Build with the schema's `row()` method, set the columns you need
        /// (typed columns accept only their own type), and pass it to
        /// `add_value()` / `add_values()`. `V` is the query's bind type and is
        /// inferred at that call.
        #[allow(dead_code)]
        #[derive(Debug, Clone)]
        pub struct $row_name<V = $crate::Value> {
            pairs: Vec<(String, V)>,
        }

        #[allow(dead_code)]
        impl $struct_name {
            /// Start an empty INSERT row for this table.
            pub fn row<V>(&self) -> $row_name<V> {
                $row_name { pairs: Vec::new() }
            }
        }

        #[allow(dead_code)]
        impl<V> $row_name<V> {
            $($crate::__qbey_schema_row_setter!(V, $($col_spec)*);)*
        }

        impl<V: Clone> $crate::ToInsertRow<V, String> for $row_name<V> {
            fn to_insert_row(&self) -> Vec<(String, V)> {
                self.pairs.clone()
            }
        }
    };
}

/// Backs the row-builder setters. Kept out of the generated struct so that
/// only the column setters occupy its namespace — a column may be called
/// `put` without colliding with anything.
///
/// Setting a column again replaces its value in place (keeping the
/// column's original position), so a default followed by a conditional
/// override does not produce a duplicate column, which `add_value()` would
/// reject.
#[doc(hidden)]
pub fn __qbey_row_put<V>(pairs: &mut Vec<(String, V)>, pair: (String, V)) {
    match pairs.iter_mut().find(|(c, _)| *c == pair.0) {
        Some(slot) => slot.1 = pair.1,
        None => pairs.push(pair),
    }
}

/// One setter on the row builder. The column is rebuilt here (bare name, no
/// table prefix) so INSERT column lists never carry the schema alias, and the
/// value goes through the same `ColumnValue` bound as `set()` / `value()`.
#[doc(hidden)]
#[macro_export]
macro_rules! __qbey_schema_row_setter {
    ($v:ident, $col:ident : $ty:ty, $sql_name:expr) => {
        pub fn $col<A>(&mut self, val: A) -> &mut Self
        where
            $crate::TypedCol<$ty>: $crate::ColumnValue<$v, A>,
        {
            let typed = $crate::TypedCol::<$ty>::new($crate::col($sql_name));
            $crate::__qbey_row_put(
                &mut self.pairs,
                $crate::ColumnValue::into_column_value(typed, val),
            );
            self
        }
    };
    ($v:ident, $col:ident : $ty:ty) => {
        pub fn $col<A>(&mut self, val: A) -> &mut Self
        where
            $crate::TypedCol<$ty>: $crate::ColumnValue<$v, A>,
        {
            let col_name = stringify!($col).trim_start_matches("r#");
            let typed = $crate::TypedCol::<$ty>::new($crate::col(col_name));
            $crate::__qbey_row_put(
                &mut self.pairs,
                $crate::ColumnValue::into_column_value(typed, val),
            );
            self
        }
    };
    ($v:ident, $col:ident, $sql_name:expr) => {
        pub fn $col<A: Into<$v>>(&mut self, val: A) -> &mut Self {
            $crate::__qbey_row_put(&mut self.pairs, ($sql_name.to_string(), val.into()));
            self
        }
    };
    ($v:ident, $col:ident) => {
        pub fn $col<A: Into<$v>>(&mut self, val: A) -> &mut Self {
            let col_name = stringify!($col).trim_start_matches("r#");
            $crate::__qbey_row_put(&mut self.pairs, (col_name.to_string(), val.into()));
            self
        }
    };
}
