---
title: Pandoc sample
bibliography: refs.bib
---
# Chemistry

Water is H~2~O and energy is mc^2^.^[Einstein, 1905.]

Term
: Its definition.

"Smart quotes" -- and dashes --- like this.

## Attributes {#attrs .lead}

A [styled link](https://example.com){.button} and `code`{.rust}, plus [small caps]{.smallcaps}
and [a *nested* span]{#n .note}.

```rust {#listing .numberLines}
fn main() {}
```

```{.plain}
no language
```

![A fjord at dawn](fjord.png){.right width=40%}

![](icon.png){.inline}

Inline image ![x](pic.png){.inline width=16} in text.

## Divs {-}

::: warning
A warning div.
:::

:::: {#outer .sidebar} ::::
Outer.

::: inner
Inner.
:::
::::

| Isotope | Mass |
|---------|------|
| H-1     | 1    |

Table: Hydrogen *isotopes* {#isotopes .center}

| A |
|---|
| 1 |

: Short caption

Closing note.[^long]

[^long]: A long footnote with a [link](https://example.com).
