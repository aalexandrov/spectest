# Feature: `meval` calculator without variables

A <em>simple</em> spec for a calculator based on the `meval` Rust crate.

## Examples with a simple context

### Background

Let's assume a very simple environment.

Given `x` as:

```
5
```

(which is a prime number);

And `y` as:

```
7
```

(which is another prime), the basic arithmetic operations can be demonstrated as follows.

### Example: Addition

When `input` is:

```
3 + x + y
```

Then `result` is:

```
15
```

<!-- ignored
### Example: Addition

When `input` is:

```
3 + x + y
```

Then `result` is:

```
15
```
-->

### Example: Subtraction

When `input` is:

```
(x * 3) - y
```

Then `result` is:

```
8
```

### Example: Multiplication

When `input` is:

```
x * 2 * y
```

Then `result` is:

```
70
```

### Example: Division

When `input` is:

```
(y * 2) / x
```

Then `result` is:

```
2.8
```

## Constant expresions

Since we started a new `h2` section, the context defined under all prior `h2`
section will be reset.

### Example: Empty context

When `input` is:

```
2 * x
```

Then `result` is:

```
cannot evaluate expression: Evaluation error: unknown variable `x`.
```

_Note_: We get this error because the **Arithmetic opeartions** background expires when we enter the **Constant Expressions** section.
