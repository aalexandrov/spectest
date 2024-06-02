# Feature: AST to AST conversion

Note: that specs support rich text formatting.
In particular, rewrites preserve _emphasized text_, **bold text** and ~strikethrough text~!

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

## Example: Constant queries (1)

When `input` is:

```sql
SELECT 1;
```

Then `output` is:

```sql
SELECT 1;
```

## Example: Linear queries (2)

When `input` is:

```sql
SELECT '_foo_' as x, '*bar*' as y;
```

Then `output` is:

```sql
SELECT
  '_foo_' as x,
  '*bar*' as y;
```
