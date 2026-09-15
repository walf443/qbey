# Changelog

## [0.4.0] - 2026-09-14

### Added

- Typed columns in `qbey_schema!`. A column declared as `name: Type` produces a `TypedCol<Type>` whose comparison methods accept only that type, so a newtype ID flows through unchanged and passing the wrong one is a compile error instead of a query that silently matches nothing. Typed, untyped and renamed (`rust_name: Type = "sql_name"`) columns can be mixed in one declaration; existing schemas are unchanged.
- `TypedCol<T>` / `TypedRhs<T>` — `eq`/`ne`/`gt`/`lt`/`gte`/`lte` against `T`, `&T` or another `TypedCol<T>` (a join between two different ID types does not compile), plus `included`/`not_included`/`between`/`not_between`/`in_range`, `like`/`not_like` on `TypedCol<String>`, and `as_`/`asc`/`desc`. `into_col()` / `as_col()` drop the type where a plain `Col` is needed.
- `ColumnValue<V, A>` — the conversion behind every `col = value` assignment. `Col` and a bare `&str` accept any `A: Into<V>`; `TypedCol<T>` accepts only `T`, `&T`, or `&str` for a `String` column.
- `UpdateQueryBuilder::set`, `InsertQuery::on_conflict_do_update` and `MysqlInsertQuery::on_duplicate_key_update` accept a `TypedCol` and type-check the value.
- `Col::value()` / `TypedCol::value()` pair a schema column with a value for an INSERT row: `ins.add_value(&[t.user_id().value(id), t.body().value("hi")])`.
- `ToInsertRow<V, C = &'static str>` — the column-name type is now a defaulted type parameter, so rows built from schema columns (`[(String, V); N]`) are accepted by `add_value()` / `add_values()` alongside `&[("name", v)]` literals. A `Vec<(C, V)>` impl was added.
- `row = Name` in `qbey_schema!` generates an INSERT row builder with one setter per column: `let mut row = t.row(); row.user_id(&id).body("hi"); ins.add_value(&row);`. Typed columns are checked like `set()` / `value()`, setters return `&mut Self` so columns can be set conditionally, setting a column again replaces the value, and the row implements `ToInsertRow<V, String>`. The name is explicit because `macro_rules!` cannot derive `CommentsRow` from `Comments`; schemas without `row = ...` are unchanged. `row` joins the schema method names a column must be renamed away from when the builder is generated.
- `set()` accepts a bare `&str` column name.
- The lint CI job now runs `cargo doc` with warnings denied.

### Changed

- **Breaking:** `UpdateQueryBuilder::set` is now `set<A>(col: impl ColumnValue<V, A>, val: A)`, and `InsertQueryBuilder::add_value` / `add_values` gained a `C: Into<String>` type parameter. External implementors of these traits must update their signatures; callers are unaffected except as noted below.
- **Breaking:** `set()` and `MysqlInsertQuery::on_duplicate_key_update()` no longer take a concrete `Col`, so `u.set("name".into(), v)` fails with `E0283: type annotations needed`. Write `col("name")`, a schema accessor, or the bare `"name"` instead.
- **Breaking:** `InsertQuery::on_conflict_do_update` is now `on_conflict_do_update<A>(columns, col: impl ColumnValue<V, A>, val: A)`. `&str` and `Col` arguments keep working unchanged.
- **Breaking:** a downstream `impl ToInsertRow<MyV> for Vec<(&'static str, MyV)>` now conflicts with the new `Vec<(C, V)>` impl; delete it, the built-in impl covers it.
- Fixed intra-doc links that rendered as plain text on docs.rs.
- Removed the unknown `package.changelog` manifest key that made cargo warn on every invocation.

### Dev Dependencies

- Updated `sqlx` to 0.9, `rusqlite` to 0.39, `testcontainers` to 0.28 (dropping `testcontainers-modules`), and replaced `ctor` with `dtor`.
- Integration tests run against both the minimum supported and the newest database versions in CI.

## [0.3.0] - 2026-04-09

### Added

- `ON CONFLICT` support for PostgreSQL and SQLite behind the `conflict` feature gate.
  - `on_conflict_do_nothing()` — `ON CONFLICT (...) DO NOTHING`.
  - `on_conflict_do_update()` — `ON CONFLICT (...) DO UPDATE SET col = ?` with a bind value.
  - `on_conflict_do_update_expr()` — `ON CONFLICT (...) DO UPDATE SET` with a raw SQL expression.
  - `on_conflict_do_update_with_excluded()` — `ON CONFLICT (...) DO UPDATE SET col = EXCLUDED.col` for convenient upsert.
- All conflict column parameters accept both `&str` and `Col` (from `qbey_schema!`). Table prefix is ignored.
- `SetClause::Excluded` variant for rendering `EXCLUDED` references in ON CONFLICT context.

## [0.2.1] - 2026-03-30

### Added

- `add_values()` method on `InsertQueryBuilder` trait for bulk row insertion. Accepts an iterator of `ToInsertRow` items, enabling ergonomic multi-row INSERTs.
- `ToInsertRow` trait documentation and `add_values` usage examples in README.

### Changed

- `add_values()` is now a default method on the `InsertQueryBuilder` trait.

### Dev Dependencies

- Updated `ctor` to 0.8.

## [0.2.0] - 2026-03-29

### Added

- `into_sql()` / `into_sql_with()` on all query types (`SelectQuery`, `InsertQuery`, `UpdateQuery`, `DeleteQuery`) and tree types (`SelectTree`, `InsertTree`, `UpdateTree`, `DeleteTree`). Consumes the query/tree to generate SQL without cloning bind values.
- `into_tree()` on all query types. Consumes the query and moves values into the AST tree instead of cloning.
- Benchmark suite for `into_sql` and render-only comparisons.

### Changed

- **Breaking:** `Dialect::placeholder()` now returns `Cow<'static, str>` instead of `String`. Fixed-string placeholders (`?` for MySQL/SQLite) no longer allocate.
- **Breaking:** `Renderer::render_select()` returns `String` instead of `(String, Vec<&V>)`. Renderers now use a bind counter (`usize`) internally instead of collecting bind references.
- **Breaking:** `render_insert()`, `render_update()`, `render_delete()` return `String` instead of `(String, Vec<&V>)`.
- **Breaking:** `render_order_by()` takes `&mut usize` instead of `&mut Vec<&V>`.
- **Breaking:** `RawSql::render()` takes `&mut usize` instead of `&mut Vec<&V>`.
- `to_sql()` / `to_sql_with()` now delegate to `into_sql_with()` internally (via `self.clone().into_sql_with()`). Existing API is unchanged.
- `to_tree()` now delegates to `into_tree()` internally (via `self.clone().into_tree()`).

### Performance

- `into_sql()` for bulk INSERT (100 rows) is ~29% faster than the previous `to_sql()` (zero bind clones).
- `to_sql()` for bulk INSERT is ~16% faster (bind counter replaces `Vec<&V>` allocation).
- Render phase for bulk INSERT is ~86% faster (bind clone elimination + `Cow` placeholders).

### Dependencies

- Updated `ctor` to 0.7.
