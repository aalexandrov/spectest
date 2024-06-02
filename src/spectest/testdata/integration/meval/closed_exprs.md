# Feature: `meval` calculator without variables

A <em>simple</em> spec for a calculator based on the `meval` Rust crate that
doesn't use variables.

## Example: Addition

When `input` is:

```
2 * 5
```

Then `result` is:

```
10
```

## Example: `pi`

When `input` is:

```
2 * pi
```

Then `result` is:

```
6.283185307179586
```

## Builtin functions

The calculator provides various builtin functions.

### Background

Given `x` as:

```
7
```

And `y` as:

```
3
```

## Example: min/max

When `input` is:

```
min(x, y)
```

Then `result` is:

```
3
```
