# TODO: Pipe Syntax Operator Coverage

SQL pipe syntax operators and their implementation status in sqipe.

Reference: [Spark SQL Pipe Syntax](https://spark.apache.org/docs/latest/sql-pipe-syntax.html), [BigQuery Pipe Syntax](https://cloud.google.com/bigquery/docs/reference/standard-sql/pipe-syntax)

## Pipe Operators

- [x] FROM / TABLE - Returns all rows from the source table
- [x] SELECT - Evaluates expressions over each row
- [ ] EXTEND - Appends new computed columns to the input table
- [ ] SET - Updates columns by replacing with new expressions
- [ ] DROP - Drops columns from the input table by name
- [ ] AS - Retains rows with a new table alias
- [x] WHERE - Returns subset of rows passing the condition
- [x] LIMIT - Returns specified number of rows
- [x] OFFSET - Skips specified number of rows
- [x] AGGREGATE - Performs aggregation with or without GROUP BY
- [ ] JOIN - Joins rows from both inputs (INNER, LEFT, RIGHT, FULL, CROSS, SEMI, ANTI)
- [x] ORDER BY - Returns rows after sorting
- [ ] UNION / INTERSECT / EXCEPT - Set operations
- [ ] TABLESAMPLE - Returns subset of rows by sampling
- [ ] PIVOT - Pivots rows to columns
- [ ] UNPIVOT - Pivots columns to rows
