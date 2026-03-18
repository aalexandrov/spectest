## Background

Given `pipeline` as:

```rust
let output = display(ast_to_ast(parse(input)));
```

And `environment` as:

```sql
CREATE TABLE s(x int, y int);
CREATE TABLE t(x int, y int);
```

and:

- `foo` as `boo`,
- `bar` as `baz`.

## Example: Constant queries (1)

When `input` is:

```sql
SELECT 1;
```

Then `output` is:

```sql
SELECT 1;
```

### Example: Linear queries (2)

When `input` is `SELECT '_foo_' as x, '*bar*' as y;` then `output` is:

```sql
SELECT
  '_foo_' as x,
  '*bar*' as y;
```
