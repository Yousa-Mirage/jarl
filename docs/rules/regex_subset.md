# regex_subset
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for indirect regex subsetting such as `x[grep(pattern, x)]`,
`x[grepl(pattern, x)]`, and their stringr equivalents.

## Why is this bad?

Functions such as `grepv()` and `stringr::str_subset()` express the intent
to return matching values directly instead of spelling out a two-step
match-and-subset operation.

This rule has an unsafe automatic fix because the replacement can change
the result. For example, subsetting a factor returns a factor, while
`grepv()` returns a character vector. `str_subset()` also drops missing
values that a logical subset would retain.

## Example

```r
x[grep(pattern, x)]
x[stringr::str_detect(x, pattern)]
```

Use instead:
```r
grepv(pattern, x)
stringr::str_subset(x, pattern)
```

For code supporting R versions before 4.5.0, the base R replacement uses
`grep(pattern, x, value = TRUE)` instead of `grepv()`.

## References

See `?grepv` and `?stringr::str_subset`
