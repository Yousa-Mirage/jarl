# empty_subset

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for empty `[]` and `[[]]` indices.
Empty `[]` used in replacement assignments, such as `x[] <- 1`, is allowed.

## Why is this bad?

`x[]` usually selects all elements of `x`, so the brackets are unnecessary.
`x[[]]` normally raises an error because no subscript is supplied.
Indices containing commas, such as `x[,]`, are not checked.

This rule has no automatic fix because custom subsetting methods can change
the behavior of `x[]` and `x[[]]`.

## Example

```r
x[]
x[[]]
```

Use instead:

```r
x
x[[1]]
```
