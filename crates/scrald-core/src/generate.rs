//! Generates large, realistic Markdown documents for benchmarks and manual
//! performance testing (DESIGN.md §4). Output is deterministic for a seed.

const WORDS: &[&str] = &[
    "the",
    "fjord",
    "winter",
    "longship",
    "skald",
    "sang",
    "of",
    "and",
    "a",
    "cold",
    "wind",
    "across",
    "grey",
    "water",
    "she",
    "he",
    "they",
    "walked",
    "toward",
    "harbor",
    "light",
    "under",
    "pale",
    "sky",
    "with",
    "salt",
    "in",
    "their",
    "hair",
    "old",
    "stories",
    "spoke",
    "iron",
    "timber",
    "rope",
    "village",
    "hearth",
    "smoke",
    "rose",
    "slowly",
    "above",
    "pines",
    "every",
    "season",
    "brought",
    "storms",
    "from",
    "north",
    "while",
    "children",
    "listened",
    "near",
    "fire",
    "remembering",
    "names",
    "carved",
    "into",
    "stone",
    "beyond",
    "hills",
    "river",
    "ran",
    "black",
    "ice",
    "cracked",
    "morning",
    "evening",
    "quiet",
    "voices",
    "sea",
];

/// A tiny deterministic pseudo-random generator (xorshift64), so the
/// generator needs no extra crate and fixtures are identical on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number in `0..n`.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn word(&mut self) -> &'static str {
        WORDS[self.below(WORDS.len())]
    }
}

/// Builds a document of roughly `target_words` words, with chapters,
/// sections, lists, tables, code, block quotes, images, and inline math.
pub fn generate_document(target_words: usize, seed: u64) -> String {
    // xorshift must not start at 0.
    let mut rng = Rng(seed.max(1));
    let mut out = String::with_capacity(target_words * 7);
    out.push_str("---\ntitle: Generated Saga\nauthor: Scrald fixture generator\ntags: [fixture, large]\n---\n\n");

    let mut words = 0;
    let mut chapter = 0;
    let mut section = 0;
    let mut paragraph = 0;
    while words < target_words {
        if paragraph % 40 == 0 {
            chapter += 1;
            out.push_str(&format!("# Chapter {chapter}: {}\n\n", title(&mut rng)));
        } else if paragraph % 8 == 0 {
            section += 1;
            out.push_str(&format!("## {} {section}\n\n", title(&mut rng)));
        }
        paragraph += 1;

        match rng.below(30) {
            0..=1 => words += list(&mut rng, &mut out),
            2 => code_block(&mut rng, &mut out),
            3 => words += table(&mut rng, &mut out),
            4..=5 => words += quote(&mut rng, &mut out),
            6 => out.push_str(&format!(
                "![Figure {paragraph}](images/figure-{}.png)\n\n",
                rng.below(20)
            )),
            _ => {
                // Rust note: `prose(&mut rng, ..., rng.below(110))` won't
                // compile: it would borrow `rng` mutably twice at once. The
                // borrow checker makes us pick the length first.
                let length = 40 + rng.below(110);
                words += prose(&mut rng, &mut out, length);
            }
        }
    }
    out
}

fn title(rng: &mut Rng) -> String {
    let mut t = String::new();
    for i in 0..2 + rng.below(3) {
        if i > 0 {
            t.push(' ');
        }
        t.push_str(rng.word());
    }
    capitalize(&t)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Writes a paragraph of about `n` words and returns the word count.
fn prose(rng: &mut Rng, out: &mut String, n: usize) -> usize {
    let mut sentence_start = true;
    for i in 0..n {
        if i > 0 {
            out.push(' ');
        }
        let word = rng.word();
        match rng.below(40) {
            0 => out.push_str(&format!("*{word}*")),
            1 => out.push_str(&format!("**{word}**")),
            2 => out.push_str(&format!("`{word}`")),
            3 => out.push_str(&format!("[{word}](https://example.com/{word})")),
            4 => out.push_str("$x_i^2$"),
            _ if sentence_start => out.push_str(&capitalize(word)),
            _ => out.push_str(word),
        }
        sentence_start = rng.below(12) == 0;
        if sentence_start {
            out.push('.');
        }
    }
    out.push_str(".\n\n");
    n
}

fn list(rng: &mut Rng, out: &mut String) -> usize {
    // One marker per list, so the whole run parses as a single list.
    let ordered = rng.below(2) == 0;
    let mut words = 0;
    for i in 1..=3 + rng.below(5) {
        if ordered {
            out.push_str(&format!("{i}. "));
        } else {
            out.push_str("- ");
        }
        let n = 3 + rng.below(10);
        for j in 0..n {
            if j > 0 {
                out.push(' ');
            }
            out.push_str(rng.word());
        }
        out.push('\n');
        words += n;
    }
    out.push('\n');
    words
}

fn table(rng: &mut Rng, out: &mut String) -> usize {
    out.push_str("| Name | Place | Season |\n|---|---|---|\n");
    let rows = 2 + rng.below(6);
    for _ in 0..rows {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            rng.word(),
            rng.word(),
            rng.word()
        ));
    }
    out.push('\n');
    rows * 3
}

fn quote(rng: &mut Rng, out: &mut String) -> usize {
    out.push_str("> ");
    let mut body = String::new();
    let length = 15 + rng.below(40);
    let n = prose(rng, &mut body, length);
    out.push_str(body.trim_end());
    out.push_str("\n\n");
    n
}

fn code_block(rng: &mut Rng, out: &mut String) {
    out.push_str("```rust\n");
    for i in 0..3 + rng.below(10) {
        out.push_str(&format!("let {}_{i} = \"{}\";\n", rng.word(), rng.word()));
    }
    out.push_str("```\n\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;
    use std::path::PathBuf;

    #[test]
    fn deterministic_for_a_seed() {
        assert_eq!(generate_document(2_000, 7), generate_document(2_000, 7));
        assert_ne!(generate_document(2_000, 7), generate_document(2_000, 8));
    }

    #[test]
    fn word_count_is_close_to_target() {
        let text = generate_document(20_000, 1);
        let doc = parse_document(PathBuf::from("gen.md"), text.as_bytes()).unwrap();
        // Prose words plus a few in headings; within 10% of the target.
        assert!(
            (18_000..=22_000).contains(&doc.word_count),
            "word count {}",
            doc.word_count
        );
        assert!(doc.toc.len() > 10);
        assert!(doc.features.has_code && doc.features.has_math);
    }
}
