---
title: Pandoc layout
scrald-flavor: pandoc
---
# Pandoc layout {#top}

![The fjord at dawn](images/fjord.png){.right width=40%}

The boats left before first light, and the fjord lay still as glass beneath
them. This paragraph wraps around the floated figure on the right, whose
caption comes from its alt text. Keep reading: the text should flow beside
the image until it runs past the bottom of it, and then continue at full
width below. Narrow the window and the figure stops floating and centers.

Sigrid counted the oars twice.^[An inline footnote: hover the number to see
it, or find it in the notes at the end.] The tide was turning, and the wind
had not yet decided which way it would blow. A longer note is
here.[^saga]

## Images {.unnumbered}

![](images/fjord.png){.left width=200}

A small image floats left, with no caption because it has no alt text. The
text wraps around it on the right side and continues below.

An inline icon ![icon](images/icon.png){.inline} sits within this sentence.

![A wide panorama, full bleed](images/wide.png){.full-bleed}

![Centered, half width](images/fjord.png){.center width=50%}

## Divs and spans

::: {.sidebar #aside}
**Sidebar.** A fenced div with an id and a class. Nested:

::: warning
An inner div with the class `warning`.
:::
:::

Some [small caps text]{.smallcaps} and an [underlined phrase]{.underline},
plus a [link with a class](https://example.com){.external}.

```python {#example .numberLines}
print("attributes on a code block")
```

## Tables

| Ship        | Oars | Crew |
|-------------|-----:|-----:|
| Sea Stallion|   60 |   70 |
| Wave Rider  |   32 |   40 |

Table: Ships of the fleet {#ships .center}

| A very wide table with many columns | Second | Third | Fourth | Fifth | Sixth | Seventh |
|---|---|---|---|---|---|---|
| Cells | that | go | on | and | on | and on |

Table: Full bleed {.full-bleed}

[^saga]: A regular footnote with *emphasis* and a second sentence, shown in
    a popover and in the endnotes.
