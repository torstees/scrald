# Grid tables and line blocks

A grid table, with a caption after it:

+---------------+---------------+--------------------+
| Fruit         | Price         | Advantages         |
+===============+==============:+:==================:+
| Bananas       | $1.34         | - built-in wrapper |
|               |               | - bright color     |
+---------------+---------------+--------------------+
| Oranges       | $2.10         | Cures scurvy.      |
|               |               |                    |
|               |               | *Tasty*, too.      |
+---------------+---------------+--------------------+

Table: Fruit and their merits {#fruit .center}

Spans and an empty cell, with no header:

+-------+-------+------+
| Two columns   | Tall |
+-------+-------+      |
| a     |       |      |
+-------+-------+------+
| Across all three     |
+----------------------+

A header and a footer:

+-------+-------+
| Ship  | Crew  |
+=======+=======+
| Raven | 40    |
+=======+=======+
| Total | 40    |
+-------+-------+

Not a table, since its lines don't meet:

+---+---+
| a  | b |
+---+---+

A line block:

| The limerick packs laughs anatomical
| Into space that is quite economical.
|    But the good ones I've seen
|    So seldom are clean
| And the clean ones so seldom are comical

| 200 Main St.
| Berkeley, CA 94718
