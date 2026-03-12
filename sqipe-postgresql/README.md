# sqipe-postgresql

PostgreSQL dialect for [sqipe](../README.md) query builder.

## Usage

```rust
use sqipe_postgresql::sqipe;
use sqipe::col;

let mut q = sqipe("employee");
q.and_where(("name", name));
q.and_where(col("age").gt(20));
q.select(&["id", "name"]);
let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE name = $1 AND age > $2"
```
