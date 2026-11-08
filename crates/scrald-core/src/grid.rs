//! Pandoc grid tables (#120): tables drawn with `+`, `-`, `=`, and `|`,
//! whose cells can hold any Markdown (several paragraphs, lists, code) and
//! span rows and columns. comrak doesn't know them, so Scrald finds them in
//! the text before parsing, like fenced divs (`pandoc.rs`):
//!
//! 1. `extract_grid_tables` reads each table's cells and blanks the table's
//!    lines to spaces (same length, line breaks kept), so the parser sees
//!    blank lines and every byte offset stays valid.
//! 2. After parsing, `insert_nodes` parses each cell's text on its own and
//!    adds the result to the document as top-level nodes, between raw HTML
//!    nodes holding the table's tags. Every one of them carries the table's
//!    source position, so they sort into place and become one block, and
//!    the usual passes (images, Pandoc spans, code highlighting) reach the
//!    cells' contents.
//!
//! ```text
//! +---------+-------------------+
//! | Fruit   | Advantages        |
//! +=========+===================+
//! | Bananas | - built-in wrapper|
//! |         | - bright color    |
//! +---------+-------------------+
//! ```
//!
//! A `=` line marks the end of the header rows; a second one, the start of
//! the footer rows. Colons in the header line (or, without a header, in the
//! top line) align columns as in pipe tables. Column widths aren't kept,
//! since the sanitizer allows no styles there; the browser sizes columns.

use comrak::nodes::{LineColumn, Node, NodeHtmlBlock, NodeValue, Sourcepos};

/// A column's alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Default,
    Left,
    Center,
    Right,
}

/// Which part of the table a row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Head,
    Body,
    Foot,
}

/// One cell: its Markdown, the first column it covers, and its spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridCell {
    pub text: String,
    pub column: usize,
    pub colspan: usize,
    pub rowspan: usize,
}

/// The cells that start in one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridRow {
    pub part: Part,
    pub cells: Vec<GridCell>,
}

/// A grid table found by `extract_grid_tables`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridTable {
    /// Byte offset of the start of the top line, in the scanned text.
    pub start: usize,
    /// Byte offset of the end of the bottom line (before its line break).
    pub end: usize,
    /// The top and bottom lines, counting from 1.
    pub first_line: usize,
    pub last_line: usize,
    /// The bottom line's last byte, counting from 1 (comrak's columns).
    pub end_column: usize,
    pub alignments: Vec<Align>,
    pub rows: Vec<GridRow>,
}

/// Whether `line` can be a grid table's top line: `+`, then runs of `-`
/// (with alignment colons), each ended by `+`, at the left margin.
pub fn is_top_border(line: &str) -> bool {
    let line = line.trim_end();
    line.len() >= 3
        && (line.starts_with("+-") || line.starts_with("+:"))
        && line.ends_with('+')
        && line.contains('-')
        && line.chars().all(|c| matches!(c, '+' | '-' | ':'))
}

/// Finds grid tables at the left margin (outside fenced code), and blanks
/// their lines to spaces. Text that looks like a table but isn't one (its
/// lines don't line up into cells) is left alone. Returns the blanked text
/// and the tables, in order.
pub fn extract_grid_tables(text: &str) -> (String, Vec<GridTable>) {
    if !text.contains("+-") && !text.contains("+:") {
        return (text.to_string(), Vec::new());
    }
    // Rust note: a tuple per line: where it starts and its text without the
    // line break. `split_inclusive` keeps each `\n`, so lengths add up.
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        lines.push((offset, line.trim_end_matches(['\n', '\r'])));
        offset += line.len();
    }

    let mut tables = Vec::new();
    let mut code_fence: Option<&str> = None;
    let mut i = 0;
    while i < lines.len() {
        let content = lines[i].1;
        let trimmed = content.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (code_fence, marker) {
            (None, Some(m)) => code_fence = Some(m),
            (Some(opened), Some(m)) if opened == m => code_fence = None,
            _ => {}
        }
        if code_fence.is_none() && is_top_border(content) {
            // Rust note: `take_while` stops at the first line that isn't
            // part of a table; `count` says how many were.
            let len = 1 + lines[i + 1..]
                .iter()
                .take_while(|(_, l)| l.starts_with('+') || l.starts_with('|'))
                .count();
            let rows: Vec<&str> = lines[i..i + len].iter().map(|(_, l)| *l).collect();
            if len >= 3
                && let Some((alignments, grid_rows)) = parse_grid(&rows)
            {
                let (start, _) = lines[i];
                let (last_start, last) = lines[i + len - 1];
                tables.push(GridTable {
                    start,
                    end: last_start + last.len(),
                    first_line: i + 1,
                    last_line: i + len,
                    end_column: last.len(),
                    alignments,
                    rows: grid_rows,
                });
                i += len;
                continue;
            }
        }
        i += 1;
    }

    let mut out = text.to_string();
    for table in &tables {
        for &(start, line) in &lines[table.first_line - 1..table.last_line] {
            // Replacing the whole line keeps its length, whatever characters
            // (multi-byte ones included) it holds.
            out.replace_range(start..start + line.len(), &" ".repeat(line.len()));
        }
    }
    (out, tables)
}

/// Reads the cells of a table's lines, or `None` if they don't form a table.
fn parse_grid(lines: &[&str]) -> Option<(Vec<Align>, Vec<GridRow>)> {
    // Columns are counted in characters, so cells can hold any text.
    let grid: Vec<Vec<char>> = lines
        .iter()
        .map(|l| l.trim_end().chars().collect())
        .collect();
    let height = grid.len();
    let width = grid.iter().map(Vec::len).max().unwrap_or(0);
    // Rust note: a closure that borrows `grid`; `get` returns `None` past
    // the end of a short line, which counts as a space.
    let at = |r: usize, c: usize| grid[r].get(c).copied().unwrap_or(' ');

    // Each cell is found from its top-left corner, in reading order, and
    // marks the area it covers (its top and left borders and inside).
    let mut covered = vec![vec![false; width]; height];
    let mut cells: Vec<(usize, usize, usize, usize)> = Vec::new();
    for r in 0..height - 1 {
        for c in 0..width.saturating_sub(1) {
            if at(r, c) != '+' || covered[r][c] {
                continue;
            }
            if let Some((bottom, right)) = scan_cell(&at, height, width, r, c) {
                for row in covered.iter_mut().take(bottom).skip(r) {
                    for spot in row.iter_mut().take(right).skip(c) {
                        *spot = true;
                    }
                }
                cells.push((r, c, bottom, right));
            }
        }
    }
    // The cells must tile the whole table, or it isn't one.
    let tiled = covered
        .iter()
        .take(height - 1)
        .all(|row| row.iter().take(width - 1).all(|&spot| spot));
    if cells.is_empty() || !tiled || at(height - 1, width - 1) != '+' {
        return None;
    }

    // Rows and columns are where cell edges fall.
    let mut row_lines: Vec<usize> = cells.iter().flat_map(|&(t, _, b, _)| [t, b]).collect();
    let mut col_lines: Vec<usize> = cells.iter().flat_map(|&(_, l, _, r)| [l, r]).collect();
    row_lines.sort_unstable();
    row_lines.dedup();
    col_lines.sort_unstable();
    col_lines.dedup();
    let row_of = |line: usize| row_lines.binary_search(&line).unwrap_or(0);
    let col_of = |col: usize| col_lines.binary_search(&col).unwrap_or(0);

    // `=` lines across the whole table end the header and start the footer.
    let full_rule = |r: usize| {
        grid[r].contains(&'=') && grid[r].iter().all(|&ch| matches!(ch, '+' | '=' | ':'))
    };
    let rules: Vec<usize> = (1..height - 1).filter(|&r| full_rule(r)).collect();
    let head_end = rules.first().map(|&r| row_of(r));
    let foot_start = rules.get(1).map(|&r| row_of(r));

    // Alignment colons are read from the header's line, else the top line.
    let align_line = rules.first().copied().unwrap_or(0);
    let alignments = col_lines
        .windows(2)
        .map(|pair| {
            let left = at(align_line, pair[0] + 1) == ':';
            let right = at(align_line, pair[1] - 1) == ':';
            match (left, right) {
                (true, true) => Align::Center,
                (true, false) => Align::Left,
                (false, true) => Align::Right,
                (false, false) => Align::Default,
            }
        })
        .collect();

    let mut rows: Vec<GridRow> = (0..row_lines.len() - 1)
        .map(|row| GridRow {
            part: match (head_end, foot_start) {
                (Some(end), _) if row < end => Part::Head,
                (_, Some(start)) if row >= start => Part::Foot,
                _ => Part::Body,
            },
            cells: Vec::new(),
        })
        .collect();
    for (top, left, bottom, right) in cells {
        let text = cell_text(&grid, top, left, bottom, right);
        let column = col_of(left);
        rows[row_of(top)].cells.push(GridCell {
            text,
            column,
            colspan: col_of(right) - column,
            rowspan: row_of(bottom) - row_of(top),
        });
    }
    for row in &mut rows {
        row.cells.sort_by_key(|cell| cell.column);
    }
    Some((alignments, rows))
}

/// The cell whose top-left corner is at (`top`, `left`), as its bottom row
/// and right column, if its borders are all there. A `+` on the way can be
/// a corner of a neighbouring cell, so the scan goes past it when the
/// border doesn't turn there.
fn scan_cell(
    at: &impl Fn(usize, usize) -> char,
    height: usize,
    width: usize,
    top: usize,
    left: usize,
) -> Option<(usize, usize)> {
    for right in left + 1..width {
        match at(top, right) {
            '+' => {
                if let Some(bottom) = scan_down(at, height, top, left, right) {
                    return Some((bottom, right));
                }
            }
            '-' | '=' | ':' => {}
            _ => return None,
        }
    }
    None
}

/// Follows a cell's right border down from its top-right corner.
fn scan_down(
    at: &impl Fn(usize, usize) -> char,
    height: usize,
    top: usize,
    left: usize,
    right: usize,
) -> Option<usize> {
    for bottom in top + 1..height {
        match at(bottom, right) {
            '+' => {
                let bottom_edge = at(bottom, left) == '+'
                    && (left + 1..right).all(|c| matches!(at(bottom, c), '-' | '=' | ':' | '+'));
                let left_edge = (top + 1..bottom).all(|r| matches!(at(r, left), '|' | '+'));
                if bottom_edge && left_edge {
                    return Some(bottom);
                }
            }
            '|' => {}
            _ => return None,
        }
    }
    None
}

/// A cell's text: the lines inside its borders, with the indent they share
/// removed, and blank lines at either end dropped.
fn cell_text(grid: &[Vec<char>], top: usize, left: usize, bottom: usize, right: usize) -> String {
    let lines: Vec<String> = grid[top + 1..bottom]
        .iter()
        .map(|row| {
            let inside: String = (left + 1..right)
                .map(|c| row.get(c).copied().unwrap_or(' '))
                .collect();
            inside.trim_end().to_string()
        })
        .collect();
    let indent = lines
        .iter()
        .filter(|l| !l.is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let lines: Vec<&str> = lines
        .iter()
        // Blank lines are empty, so `get` keeps them as "".
        .map(|l| l.get(indent..).unwrap_or(""))
        .collect();
    lines.join("\n").trim_matches('\n').to_string()
}

/// Adds each table to `root` as raw HTML nodes for its tags and the parsed
/// contents of its cells, all positioned at the table's lines.
// Rust note: `'a` ties the new nodes to the arena that owns the document's
// other nodes, so they can join its tree and live as long as it does.
pub fn insert_nodes<'a>(
    arena: &'a comrak::Arena<'a>,
    root: Node<'a>,
    tables: &[GridTable],
    options: &comrak::Options,
) {
    for table in tables {
        let pos = Sourcepos {
            start: LineColumn {
                line: table.first_line,
                column: 1,
            },
            end: LineColumn {
                line: table.last_line,
                column: table.end_column.max(1),
            },
        };
        // Tags waiting to be written as one raw HTML node.
        let mut tags = String::from("<table>\n");
        let flush = |tags: &mut String| {
            if tags.is_empty() {
                return;
            }
            let node = arena.alloc(
                NodeValue::HtmlBlock(NodeHtmlBlock {
                    block_type: 0,
                    literal: std::mem::take(tags),
                })
                .into(),
            );
            node.data_mut().sourcepos = pos;
            root.append(node);
        };
        let mut part: Option<Part> = None;
        for row in &table.rows {
            if part != Some(row.part) {
                if let Some(open) = part {
                    tags.push_str(part_tag(open, true));
                }
                tags.push_str(part_tag(row.part, false));
                part = Some(row.part);
            }
            tags.push_str("<tr>\n");
            for cell in &row.cells {
                let name = if row.part == Part::Head { "th" } else { "td" };
                tags.push_str(&cell_open_tag(name, cell, table));
                let doc = comrak::parse_document(arena, &cell.text, options);
                // A footnote can't be defined inside a cell: it would leave
                // the table. Its reference stays as text.
                let children: Vec<Node<'a>> = doc
                    .children()
                    .filter(|n| !matches!(n.data().value, NodeValue::FootnoteDefinition(_)))
                    .collect();
                if !children.is_empty() {
                    flush(&mut tags);
                }
                for child in children {
                    child.detach();
                    for node in child.descendants() {
                        node.data_mut().sourcepos = pos;
                    }
                    root.append(child);
                }
                tags.push_str(&format!("</{name}>\n"));
            }
            tags.push_str("</tr>\n");
        }
        if let Some(open) = part {
            tags.push_str(part_tag(open, true));
        }
        tags.push_str("</table>\n");
        flush(&mut tags);
    }
}

fn part_tag(part: Part, close: bool) -> &'static str {
    match (part, close) {
        (Part::Head, false) => "<thead>\n",
        (Part::Head, true) => "</thead>\n",
        (Part::Body, false) => "<tbody>\n",
        (Part::Body, true) => "</tbody>\n",
        (Part::Foot, false) => "<tfoot>\n",
        (Part::Foot, true) => "</tfoot>\n",
    }
}

fn cell_open_tag(name: &str, cell: &GridCell, table: &GridTable) -> String {
    let align = match table.alignments.get(cell.column) {
        Some(Align::Left) => " align=\"left\"",
        Some(Align::Center) => " align=\"center\"",
        Some(Align::Right) => " align=\"right\"",
        _ => "",
    };
    let mut tag = format!("<{name}{align}");
    if cell.colspan > 1 {
        tag.push_str(&format!(" colspan=\"{}\"", cell.colspan));
    }
    if cell.rowspan > 1 {
        tag.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
    }
    tag.push('>');
    tag
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(table: &GridTable) -> Vec<Vec<&str>> {
        table
            .rows
            .iter()
            .map(|row| row.cells.iter().map(|c| c.text.as_str()).collect())
            .collect()
    }

    const FRUIT: &str = "\
+---------------+---------------+--------------------+
| Fruit         | Price         | Advantages         |
+===============+==============:+:==================:+
| Bananas       | $1.34         | - built-in wrapper |
|               |               | - bright color     |
+---------------+---------------+--------------------+
| Oranges       | $2.10         | - cures scurvy     |
|               |               | - tasty            |
+---------------+---------------+--------------------+
";

    #[test]
    fn cells_rows_header_and_alignment() {
        let text = format!("Before.\n\n{FRUIT}\nAfter.\n");
        let (blanked, tables) = extract_grid_tables(&text);
        assert_eq!(blanked.len(), text.len());
        assert_eq!(blanked.lines().count(), text.lines().count());
        assert!(blanked.contains("Before.") && blanked.contains("After."));
        assert!(!blanked.contains('+') && !blanked.contains('|'));

        let [table] = tables.as_slice() else {
            panic!("one table: {tables:?}");
        };
        assert_eq!((table.first_line, table.last_line), (3, 11));
        assert_eq!(&text[table.start..table.end], FRUIT.trim_end());
        assert_eq!(table.end_column, 54);
        assert_eq!(
            table.alignments,
            [Align::Default, Align::Right, Align::Center]
        );
        assert_eq!(
            texts(table),
            [
                vec!["Fruit", "Price", "Advantages"],
                vec!["Bananas", "$1.34", "- built-in wrapper\n- bright color"],
                vec!["Oranges", "$2.10", "- cures scurvy\n- tasty"],
            ]
        );
        let parts: Vec<Part> = table.rows.iter().map(|r| r.part).collect();
        assert_eq!(parts, [Part::Head, Part::Body, Part::Body]);
    }

    #[test]
    fn spans_footer_and_no_header() {
        let text = "\
+-----+-----+------+
| one cell  | c    |
+-----+-----+      |
| a   | b   |      |
+:====+====:+======+
| foot             |
+------------------+
";
        let (_, tables) = extract_grid_tables(text);
        let [table] = tables.as_slice() else {
            panic!("one table: {tables:?}");
        };
        assert_eq!(
            table.alignments,
            [Align::Left, Align::Right, Align::Default]
        );
        assert_eq!(
            texts(table),
            [vec!["one cell", "c"], vec!["a", "b"], vec!["foot"]]
        );
        let first = &table.rows[0].cells;
        assert_eq!((first[0].colspan, first[0].rowspan), (2, 1));
        assert_eq!((first[1].column, first[1].rowspan), (2, 2));
        assert_eq!(table.rows[2].cells[0].colspan, 3);
        // One `=` line is a header, so the rows above it are the header.
        let parts: Vec<Part> = table.rows.iter().map(|r| r.part).collect();
        assert_eq!(parts, [Part::Head, Part::Head, Part::Body]);
    }

    #[test]
    fn header_and_footer() {
        let text = "\
+---+---+
| h | i |
+===+===+
| a | b |
+===+===+
| f | g |
+---+---+
";
        let (_, tables) = extract_grid_tables(text);
        let parts: Vec<Part> = tables[0].rows.iter().map(|r| r.part).collect();
        assert_eq!(parts, [Part::Head, Part::Body, Part::Foot]);
    }

    #[test]
    fn multibyte_text_and_crlf() {
        let text = "+------+----+\r\n| Søren| ½  |\r\n+------+----+\r\n";
        let (blanked, tables) = extract_grid_tables(text);
        assert_eq!(blanked.len(), text.len());
        assert!(blanked.contains("\r\n"));
        assert_eq!(texts(&tables[0]), [vec!["Søren", "½"]]);
        assert_eq!(&text[tables[0].start..tables[0].end], text.trim_end());
    }

    #[test]
    fn things_that_arent_tables_are_left_alone() {
        for text in [
            // Lines that don't line up.
            "+---+---+\n| a  | b |\n+---+---+\n",
            // No bottom border.
            "+---+\n| a |\n| b |\n",
            // Indented, so it's code.
            "    +---+\n    | a |\n    +---+\n",
            // Inside fenced code.
            "```\n+---+\n| a |\n+---+\n```\n",
            // A rule with text after it.
            "+---+ more\n| a |\n+---+\n",
        ] {
            let (blanked, tables) = extract_grid_tables(text);
            assert!(tables.is_empty(), "{text:?}: {tables:?}");
            assert_eq!(blanked, text);
        }
    }

    #[test]
    fn top_border() {
        assert!(is_top_border("+---+:--:+"));
        assert!(is_top_border("+-+  "));
        assert!(!is_top_border("+===+"));
        assert!(!is_top_border(" +---+"));
        assert!(!is_top_border("+---"));
        assert!(!is_top_border("+ - +"));
    }
}
