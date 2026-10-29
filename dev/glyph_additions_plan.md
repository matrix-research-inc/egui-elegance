# Glyph Additions — Proposal

> **Status:** shipped, and then expanded. This document proposed the first six action glyphs (`U+E00C`–`U+E011`). A follow-up batch added 17 more at `U+E012`–`U+E022` — a Status family (`info`, `triangle-alert`, `circle-x`, `circle-check`) that also rewires `CalloutTone`'s per-tone marks, plus broadly-useful nav/action icons (`settings`, `menu`, `arrow-left`/`right`, `external-link`, `chevron-right`/`down`) and situational ones (`funnel`→`FILTER`, `eye`, `eye-off`, `house`→`HOME`, `lock`, `save`). `folder` and `folder-open` landed later at `U+E023`–`U+E024`, on a downstream report of falling back to the 📁 emoji for a breadcrumb root — the emoji renders in the system emoji font, so it reads as a different icon set beside the Lucide glyphs next to it. `play`, `pause` and `square`→`STOP` followed at `U+E025`–`U+E027`, on a downstream report of using the base-font `▶`/`■` for run controls, the same seam. The conventions and process below applied unchanged to every batch.

A proposal to extend the bundled Elegance Symbols icon set with six common action glyphs. The current set covers transfer (`upload`/`download`), search/zoom, status (`circle-alert`, `check`, `x`), and a few object icons (`pin`, `copy`, `network`, `power`, `trash-2`, `pencil`), but downstream apps keep reaching for a small group of everyday action icons that aren't there yet and have to fall back to text-only buttons or base-font literals.

---

## 1. Motivation

The bundled glyphs were chosen to support the widgets shipped in elegance itself (the `upload` glyph backs `FileDropZone`, the status marks back `Callout`/`StatusPill`, and so on). They were never meant to be a complete UI icon font, and this proposal does not try to make them one.

But once an app standardises on elegance buttons, it wants its *own* action buttons to carry the same Lucide-styled icons rather than mixing iconless text buttons with elegance's iconed ones. A recurring set of generic actions has no glyph today:

- **Add / create** — "Add item", "New", "+". Currently text-only.
- **Open a terminal / console** — launching an interactive shell session.
- **Keys / credentials** — anything key- or secret-related (a natural fit for a library that already ships `power`, `network`, and `pin`).
- **Refresh / reload** — re-fetch or re-scan, distinct from a navigational arrow.
- **Sort direction** — an ascending/descending toggle next to a sort control. There is no directional arrow in the set at all today, so apps fall back to the base-font `\u{2191}`/`\u{2193}` literals, which don't carry the Lucide styling and sit on a different baseline/weight than the neighbouring iconed buttons.

Each of these is generic enough to recur across unrelated apps. Adding them to the shared font keeps icon styling consistent and saves every consumer from shipping a parallel icon font just to fill the gaps.

This list is deliberately tight. Glyphs that read as one app's domain vocabulary (e.g. a `server` icon for "hosts") or whose meaning is ambiguous without a label (e.g. `layers` for "apply to all") were considered and left out — see [Considered and dropped](#6-considered-and-dropped).

## 2. Proposed Glyphs

All sourced from [Lucide](https://lucide.dev) to match the existing set and stay within the project's license story (Lucide is MIT/ISC, compatible with elegance's MIT/Apache-2.0 dual license). Codepoints continue the Private Use Area sequence from the current maximum (`PENCIL = U+E00B`), contiguously and without gaps.

| Const        | Lucide name  | Codepoint | Purpose                                                        |
| ------------ | ------------ | --------- | ------------------------------------------------------------- |
| `PLUS`       | `plus`       | `U+E00C`  | Add / create / "new" actions                                  |
| `TERMINAL`   | `terminal`   | `U+E00D`  | Open a shell / console session                                |
| `KEY`        | `key-round`  | `U+E00E`  | Keys, credentials, secrets                                    |
| `REFRESH`    | `refresh-cw` | `U+E00F`  | Reload / re-scan (distinct from a navigation arrow)           |
| `ARROW_UP`   | `arrow-up`   | `U+E010`  | Ascending sort direction; also generic "move up" / navigation |
| `ARROW_DOWN` | `arrow-down` | `U+E011`  | Descending sort direction; also generic "move down" / nav     |

Notes:

- `KEY` is baked from `key-round` rather than plain `key` because the rounded bow reads more clearly at small button sizes. (The const name follows the suffix-dropping convention in §4, not the Lucide variant name.)
- `ARROW_UP`/`ARROW_DOWN` are a deliberate pair: they fix the sort-toggle baseline seam called out in §1 *and* double as generic directional arrows for reorder and navigation, so they earn their place by reuse. Lucide's sort-specific marks (`arrow-up-narrow-wide` / `arrow-down-wide-narrow`) read more explicitly as "sort" but are single-purpose; a combined `arrow-up-down` toggle can't show which direction is currently active. The plain pair beats both.
- Exact Lucide names should be confirmed against the pinned tag (currently `1.11.0`) before baking. The bake script fetches each name from that release, so a renamed or missing icon fails loudly with a 404 rather than silently producing tofu — see §3.

## 3. Implementation

Adding a glyph touches three lists that must stay in sync, then a font rebake. None of this is new machinery; it's the same path the existing icons took. The rebake is low-risk by construction: the script fetches each icon from a *pinned* Lucide tag, so it is reproducible, and a renamed or missing icon 404s loudly rather than silently baking nothing.

0. **Add a sync check (do this first, independent of the new glyphs).** Today the only thing tying `LUCIDE_GLYPHS` in the bake script to the `glyphs` consts in `src/lib.rs` is a comment. With six entries about to be added, that should become an enforced invariant: a test or CI step that asserts the const codepoints exactly match the baked set. This also protects the *existing* glyphs, so it's worth landing even on its own.

1. **`scripts/update_lucide_glyphs.py`** — append `(lucide_name, codepoint)` entries to `LUCIDE_GLYPHS`, keeping codepoints contiguous from `0xE00C`.

2. **`src/lib.rs`** — add a matching `pub const` to the `glyphs` module with a doc comment linking the Lucide source, mirroring the existing entries:

   ```rust
   /// Plus / add icon. Source: [Lucide `plus`](https://lucide.dev/icons/plus).
   pub const PLUS: char = '\u{E00C}';
   ```

3. **`examples/render_docs.rs`** — add the new constants to the `icons` array in `render_glyphs()` so they appear in the glyph gallery, then regenerate `docs/images/glyphs.png` (and any README reference to it). Because a binary `.ttf` diff is unreviewable, this regenerated PNG *is* the human-reviewable form of the font change — eyeball the before/after.

4. **Rebake the font** — run `python3 scripts/update_lucide_glyphs.py` from the repo root (deps: `pip install picosvg fonttools`; needs network access to fetch the pinned Lucide tag). This rewrites `assets/elegance-symbols.ttf` in place, replacing glyphs at existing codepoints and appending the new ones. Commit the regenerated `.ttf`.

5. **Verify** — `cargo run --example render_docs` and confirm the new glyphs sit on the same baseline and advance width as the existing icons (the script bakes them at the shared `CAP_HEIGHT` / `ADVANCE`, so this should be automatic), and that the sync check from step 0 passes.

## 4. Conventions This Follows

- **Private Use Area, contiguous.** Icons with no standard Unicode codepoint live at `U+E000+` and are assigned in order; this proposal continues that run without gaps. (No reserved boundaries — there's a single writer and no grouping that a gap would buy.)
- **Lucide-sourced, pinned.** Every glyph traces to a Lucide icon at the pinned release tag, so the bake stays reproducible and the license stays clean.
- **Const name = Lucide name with disambiguating suffix dropped.** This is the existing de-facto rule: `TRASH` from `trash-2`, alongside `PENCIL`, `NETWORK`, `POWER`. By that precedent the new consts are `REFRESH` (from `refresh-cw`) and `KEY` (from `key-round`), not `REFRESH_CW` / `KEY_ROUND`; the rest (`PLUS`, `TERMINAL`, `ARROW_UP`, `ARROW_DOWN`) carry no suffix to drop.
- **Doc comment links the source.** Each `pub const` cites its Lucide icon page, matching the existing entries.
- **No new public surface beyond consts.** These are plain `char` constants in the existing `glyphs` module; nothing else in the API changes.

## 5. Decisions

The choices that were open during drafting, now settled:

1. **Scope.** Bake these six. The four action glyphs (`PLUS`, `TERMINAL`, `KEY`, `REFRESH`) are the ones nearly every non-trivial app hits; the `ARROW_UP`/`ARROW_DOWN` pair is included because it is broadly reusable and fixes a concrete styling seam, not because sort alone justifies it.
2. **`key` vs `key-round`.** Bake `key-round` (clearer rounded bow at button size); expose it as `KEY`.
3. **Naming.** Intent-neutral, following the existing suffix-dropping rule (see §4): `REFRESH`, `KEY`.
4. **Codepoints.** Contiguous from `U+E00C`, no reserved gaps.
5. **Sort direction.** The reusable `arrow-up` / `arrow-down` pair, over the sort-specific marks or a combined toggle.

## 6. Considered and Dropped

Kept here so the reasoning isn't relitigated later:

- **`server`** — proposed for "Add host" / infrastructure rows, but that's one app's domain vocabulary rather than a generic UI action. The library-scope bar is two unrelated use cases before generalizing; this had one. Add it if a second app genuinely needs it.
- **`layers` (bulk / "apply to all")** — `layers` reads as stacking or z-order, not "do this to every row," so it would need a label to be understood. Once a label is present, the simpler convention wins: reuse the single-item glyph (`upload`/`download`) with a count in the label, e.g. "Install on 4 hosts." No dedicated glyph baked.
