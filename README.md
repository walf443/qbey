
# sqipe

pipe syntax based sql query builder

> **Note:** This project is in early development. APIs may change without backward compatibility.

## SYNOPSIS

### Basic usage

```rust
let mut q = sqipe("employee");
q.and_where(("name", name));   // tuple shorthand for Eq
q.select(&["id", "name"]);

// Standard SQL (default placeholder: ?)
let (sql, binds) = q.to_sql();
// => "SELECT "id", "name" FROM "employee" WHERE "name" = ?"

// Pipe syntax SQL (default placeholder: ?)
let (sql, binds) = q.to_pipe_sql();
// => "FROM "employee" |> WHERE "name" = ? |> SELECT "id", "name""
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
// => "SELECT `id`, `name` FROM `employee` WHERE `name` = ?"
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

### BETWEEN

```rust
let mut q = sqipe("employee");
q.and_where(col("age").between(20, 30));
q.select(&["id", "name"]);

let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE age BETWEEN ? AND ?"
```

### Range conditions

Rust range types are automatically converted to the appropriate SQL conditions.

```rust
// Inclusive range: BETWEEN
q.and_where(col("age").in_range(20..=30));
// => "age BETWEEN ? AND ?"

// Exclusive range: >= AND <
q.and_where(col("age").in_range(20..30));
// => "age >= ? AND age < ?"

// From range: >=
q.and_where(col("age").in_range(20..));
// => "age >= ?"

// To range: <
q.and_where(col("age").in_range(..30));
// => "age < ?"

// To inclusive range: <=
q.and_where(col("age").in_range(..=30));
// => "age <= ?"
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

### Aggregate / GROUP BY

```rust
use sqipe::aggregate;

let mut q = sqipe("employee");
q.aggregate(&[
    aggregate::count_all().as_("cnt"),
    aggregate::sum("salary").as_("total_salary"),
]);
q.group_by(&["dept"]);

let (sql, binds) = q.to_sql();
// => "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total_salary FROM employee GROUP BY dept"

let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> AGGREGATE COUNT(*) AS cnt, SUM(salary) AS total_salary GROUP BY dept"
```

Available aggregate functions: `count_all()`, `count(col)`, `sum(col)`, `avg(col)`, `min(col)`, `max(col)`, `expr(raw_sql)`.

### HAVING

`and_where` / `or_where` called after `aggregate()` automatically become HAVING conditions.

```rust
let mut q = sqipe("employee");
q.and_where(col("active").eq(true));       // WHERE (before aggregate)
q.aggregate(&[aggregate::count_all().as_("cnt")]);
q.group_by(&["dept"]);
q.and_where(col("cnt").gt(5));             // HAVING (after aggregate)

let (sql, binds) = q.to_sql();
// => "SELECT dept, COUNT(*) AS cnt FROM employee WHERE active = ? GROUP BY dept HAVING cnt > ?"

let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> WHERE active = ? |> AGGREGATE COUNT(*) AS cnt GROUP BY dept |> WHERE cnt > ?"
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

### UNION / UNION ALL

```rust
let mut q1 = sqipe("employee");
q1.and_where(("dept", "eng"));
q1.select(&["id", "name"]);

let mut q2 = sqipe("employee");
q2.and_where(("dept", "sales"));
q2.select(&["id", "name"]);

let uq = q1.union_all(&q2);
let (sql, binds) = uq.to_sql();
// => "SELECT id, name FROM employee WHERE dept = ? UNION ALL SELECT id, name FROM employee WHERE dept = ?"

// With ORDER BY and LIMIT on the union result
let mut uq = q1.union_all(&q2);
uq.order_by(col("name").asc());
uq.limit(10);
let (sql, binds) = uq.to_sql();
// => "... UNION ALL ... ORDER BY name ASC LIMIT 10"
```

### MySQL-specific features (sqipe-mysql)

```rust
use sqipe_mysql::sqipe;

let mut q = sqipe("employee");
q.force_index(&["idx_name"]);
q.and_where(("name", "Alice"));
q.select(&["id", "name"]);

let (sql, binds) = q.to_sql();
// => "SELECT `id`, `name` FROM `employee` FORCE INDEX (idx_name) WHERE `name` = ?"

// Also available: use_index, ignore_index
```
