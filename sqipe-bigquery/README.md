# sqipe-bigquery

BigQuery dialect for [sqipe](../README.md) query builder.

## Usage

```rust
use sqipe_bigquery::sqipe;
use sqipe::col;

let mut q = sqipe("employee");
q.and_where(("name", name));
q.select(&["id", "name"]);

// Pipe syntax SQL (BigQuery supports pipe syntax natively)
let (sql, binds) = q.to_pipe_sql();
// => "FROM employee |> WHERE name = @p1 |> SELECT id, name"

// Standard SQL
let (sql, binds) = q.to_sql();
// => "SELECT id, name FROM employee WHERE name = @p1"
```
