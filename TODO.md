# Scrald: TODO

Work is tracked in [GitHub issues](https://github.com/torstees/scrald/issues). Each milestone is a parent issue whose sub-issues are the individual work items, and each has a matching [GitHub Milestone](https://github.com/torstees/scrald/milestones). Milestones are ordered so that each one leaves a usable, testable app. See `AGENTS.md` ("Work tracking") for how to add and close items.

This file keeps only the index below and the decisions log.

---

## Milestones

| Milestone | Issue | Design |
|---|---|---|
| [M0: Scaffolding](https://github.com/torstees/scrald/milestone/1) | [#1](https://github.com/torstees/scrald/issues/1) |  |
| [M1: Core parsing and rendering](https://github.com/torstees/scrald/milestone/2) | [#10](https://github.com/torstees/scrald/issues/10) | §3, §5 |
| [M2: Reader shell](https://github.com/torstees/scrald/milestone/3) | [#25](https://github.com/torstees/scrald/issues/25) | §4, §6, §10, §11 |
| [M3: Themes, typography, and memory](https://github.com/torstees/scrald/milestone/4) | [#39](https://github.com/torstees/scrald/issues/39) | §7, §12 |
| [M4: Flavors and rich content](https://github.com/torstees/scrald/milestone/5) | [#56](https://github.com/torstees/scrald/issues/56) | §5, §6.3 |
| [M5: Front matter panel](https://github.com/torstees/scrald/milestone/6) | [#72](https://github.com/torstees/scrald/issues/72) | §8 |
| [M6: Editing](https://github.com/torstees/scrald/milestone/7) | [#79](https://github.com/torstees/scrald/issues/79) | §9 |
| [M7: Startup and integration](https://github.com/torstees/scrald/milestone/8) | [#89](https://github.com/torstees/scrald/issues/89) | §11 |
| [M8: Fonts and theme creation](https://github.com/torstees/scrald/milestone/9) | [#94](https://github.com/torstees/scrald/issues/94) | §7.3, §7.5 |
| [M9: Polish](https://github.com/torstees/scrald/milestone/10) | [#100](https://github.com/torstees/scrald/issues/100) |  |
| [M10: macOS and Linux (final v1 milestone)](https://github.com/torstees/scrald/milestone/11) | [#107](https://github.com/torstees/scrald/issues/107) | §11.1 |
| [Later / ideas](https://github.com/torstees/scrald/milestone/12) | [#116](https://github.com/torstees/scrald/issues/116) |  |

---

## Decisions log

Record notable decisions made during implementation here (date, decision, reason), so `DESIGN.md` can be updated to match.

| Date | Decision | Reason |
|---|---|---|
| 2026-10-08 | Front matter settings use flat `scrald-theme` / `scrald-flavor` keys; nested `scrald:` map is read but never written (DESIGN §8.1, §15 Q1) | Obsidian's Properties panel can't display or edit nested maps; flat keys keep the minimal-edit YAML engine to top-level scalars |
| 2026-10-08 | Autosave off by default for v1; add close prompt and crash-recovery files in app data (DESIGN §9.3, §12, §15 Q2) | New edit/save path shouldn't write to user files unattended; recovery files cover crashes without touching the document |
| 2026-10-08 | Note transclusion is post-v1; v1 renders `![[Other note]]` as a link card (DESIGN §5.3, §15 Q3) | Keeps v1 scope down; transclusion raises cross-file source-range, editing, and cycle questions worth designing properly |
| 2026-10-08 | Windows first; macOS and Linux support is the final v1 milestone, M10 (DESIGN §11.1, §15 Q4) | Focus early work on one platform while keeping code portable so the port is mostly verification and packaging |
| 2026-10-08 | LF line endings for all repo files, enforced by `.gitattributes` (`eol=lf`) and `.editorconfig`; test fixtures exempt (`-text`) | Maintainer preference; also keeps diffs clean across platforms. Fixtures must keep CRLF/BOM bytes to test preservation |
| 2026-10-08 | `scrald-core` is tested on Windows, macOS, and Linux in CI from M0 (DESIGN §11.1) | Core has no GUI dependencies, so cross-platform CI is nearly free and catches portability slips early |
| 2026-10-08 | Work items tracked as GitHub issues: one parent issue per milestone with sub-issues, plus matching GitHub Milestones; TODO.md reduced to an index and this log | Maintainer already works this way and follows progress in a GitHub Project |
