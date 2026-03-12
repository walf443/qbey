
# sqipe

pipe syntax based sql query builder

## SYNOPSIS

### Basic usage

```rust
let mut q = sqipe("employee");
q.and_where(("name", name));   // tuple shorthand for Eq
q.select(&["id", "name"]);

// Standard SQL (default placeholder: ?)
let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE name = ?"

// Pipe syntax SQL (default placeholder: ?)
let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> WHERE name = ? |> SELECT id, name"
```

### Dialect support

Each dialect is a separate crate with its own `sqipe` function.
Dialect-specific methods are available through the wrapper.

```rust
// MySQL (sqipe-mysql)
use sqipe_mysql::sqipe;
let mut q = sqipe("employee");
q.and_where(("name", name));
q.select(&["id", "name"]);
let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE name = ?"

// PostgreSQL (sqipe-postgresql)
use sqipe_postgresql::sqipe;
let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE name = $1"

// BigQuery (sqipe-bigquery)
use sqipe_bigquery::sqipe;
let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> WHERE name = @p1 |> SELECT id, name"
```

### Comparison operators

```rust
let mut q = sqipe("employee");
q.and_where(("name", name));               // tuple shorthand for Eq
q.and_where(col("age").gt(min_age));        // age > ?
q.and_where(col("age").lte(max_age));       // age <= ?
q.and_where(col("salary").lt(max_salary));  // salary < ?
q.and_where(col("level").gte(min_level));   // level >= ?
q.and_where(col("role").ne("intern"));      // role != ?
q.select(&["id", "name"]);

let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE name = ? AND age > ? AND age <= ? AND salary < ? AND level >= ? AND role != ?"
```

### Dynamic query building

```rust
let mut q = sqipe("employee");

if let Some(name) = name {
    q.and_where(("name", name));
}
if let Some(min_age) = min_age {
    q.and_where(col("age").gt(min_age));
}

q.select(&["id", "name"]);
let (sql, binds) = q.to_sql();
```

### or_where

```rust
// Simple OR
let mut q = sqipe("employee");
q.and_where(("name", name));
q.or_where(col("role").eq("admin"));
let (sql, binds) = q.to_sql();
// => "SELECT * FROM employee WHERE name = ? OR role = ?"
```

### Grouping conditions with any / all

```rust
let mut q = sqipe("employee");
q.and_where(("name", name));
q.and_where(any(col("role").eq("admin"), col("role").eq("manager")));
let (sql, binds) = q.to_sql();
// => "SELECT * FROM employee WHERE name = ? AND (role = ? OR role = ?)"

// Combining all + any
let mut q = sqipe("employee");
q.and_where(
    any(
        all(col("role").eq("admin"), col("dept").eq("eng")),
        all(col("role").eq("manager"), col("dept").eq("sales")),
    )
);
let (sql, binds) = q.to_sql();
// => "SELECT * FROM employee WHERE (role = ? AND dept = ?) OR (role = ? AND dept = ?)"
```

### Order By

```rust
let mut q = sqipe("employee");
q.select(&["id", "name", "age"]);
q.order_by(col("name").asc());
q.order_by(col("age").desc());

let (sql, binds) = q.to_sql();
// => "SELECT id, name, age FROM employee ORDER BY name ASC, age DESC"

let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> SELECT id, name, age |> ORDER BY name ASC, age DESC"
```

### Limit / Offset

```rust
let mut q = sqipe("employee");
q.select(&["id", "name"]);
q.limit(10);
q.offset(20);

let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee LIMIT 10 OFFSET 20"

let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> SELECT id, name |> LIMIT 10 OFFSET 20"
```

### Method chaining

```rust
let (sql, binds) = sqipe("employee")
    .and_where(("name", name))
    .and_where(col("age").gt(20))
    .select(&["id", "name"])
    .to_sql();
// => "SELECT id, name FROM employee WHERE name = ? AND age > ?"
```
