---
name: i18n
description: >-
  Guide for adding or modifying languages in edif.io — viewer/UI languages
  (Paraglide) and per-mode content languages (adapter word sets). Use when asked
  to add, translate, or localize a language.
---

# Internationalization

edif.io has **two independent localization axes**. A language existing on one
does **not** imply it exists on the other — never assume parity.

- **Viewer language** (per-player, frontend): every piece of UI text, including
  adapter "chrome" (mode names, option labels, select choices, input
  placeholders). Powered by [Paraglide JS](https://inlang.com/m/gerre34r/library-inlang-paraglideJs).
  The player picks it; it persists to `localStorage` and re-renders the app.
- **Content language** (per-mode, backend): the actual challenge content, e.g.
  the keyboarding **Word Set** the host selects. Rides the existing
  `game_options` plumbing — no protocol change.

## Architecture

### Viewer language (frontend)

- `client/project.inlang/settings.json` — Paraglide config: `baseLocale` (`en`),
  the `locales` list, and the message-format plugin (loaded from
  `node_modules`, **not** a CDN, so builds are hermetic).
- `client/messages/<locale>.json` — one catalog per locale. `en.json` is the
  hand-authored **source of truth**. Every catalog must define the **exact same
  key set** as `en.json`.
- `client/messages/l33t.json` — **GENERATED**, do not hand-edit. Produced from
  `en.json` by `npm run leetify` (`client/scripts/leetify.mjs` + the pure
  transform in `client/src/lib/i18n/leetify.js`).
- `client/src/lib/paraglide/` — **GENERATED & gitignored** Paraglide output
  (`npm run paraglide:compile`). Never edit. Messages are consumed via
  `import { m } from '$lib/paraglide/messages'`, called as `m.some_key()`.
- `client/src/lib/components/LanguageSwitcher.svelte` — the picker. Derives each
  language's display name dynamically from `lang_name_<locale>`, so **adding a
  language needs no edit here.**
- `client/src/lib/i18n/chrome.ts` — localizes adapter chrome (which arrives from
  the backend in English) keyed by stable ids (`gameKey` / option `key` / choice
  `value`), falling back to the server's English label for any key a catalog
  lacks. **Also needs no edit to add a language** — it just reads whatever keys
  exist.
- `client/src/lib/i18n/coverage.test.ts` — auto-discovers every catalog and
  fails if any locale's key set differs from `en`. New languages are covered
  automatically.

### Content language (backend)

- Each adapter exposes content choices through its `option_schema()`
  `OptionFieldKind::Select`. Example: `adapters/keyboarding/src/lib.rs` exposes a
  `wordSet` select (`english` / `l33t`) and switches on it in `next_prompt`.

## Recipe: add a viewer (UI) language

Worked example: **Spanish (`es`)**. Substitute your locale code (a simple,
lowercase token — avoid hyphens; message keys can't contain them).

1. **Register the locale.** Add it to `locales` in
   `client/project.inlang/settings.json`:
   ```json
   "locales": ["en", "l33t", "es"]
   ```

2. **Add the endonym to the source catalog.** In `client/messages/en.json` add a
   `lang_name_<locale>` key whose value is the language's *own* name (the
   endonym, shown in the picker for every viewer):
   ```json
   "lang_name_es": "Español"
   ```

3. **Create `client/messages/es.json`.** Copy `en.json` verbatim, keep the
   `$schema` line and **every** key (same set — no more, no fewer), then
   translate the string *values*. Rules:
   - Preserve interpolation placeholders **exactly**: `{winner}`, `{amount}`,
     `{player}`, `{label}`, `{id}`, `{url}`. Reorder words around them as the
     grammar needs, but never rename or drop a placeholder.
   - Keep `**bold**` markers around the equivalent translated word(s)
     (e.g. `rules_grow`, `rules_winner`).
   - `lang_name_*` values are endonyms and are therefore the **same in every
     catalog** (`lang_name_en` → `"English"`, `lang_name_es` → `"Español"`, …).
   - Translate the adapter chrome keys too (`adapter_*`, `placeholder_*`,
     `opt_*`, `choice_*`). If a term has no good translation, copy the English —
     but the key must be present (parity is enforced).

4. **Regenerate the l33t catalog.** `en.json` gained a key in step 2, so:
   ```sh
   cd client && npm run leetify      # or: make generate
   ```
   This adds the leetified `lang_name_es` to `l33t.json`, keeping all catalogs in
   parity. (`npm run leetify:check` in CI fails otherwise.)

5. **Verify.** From the repo root:
   ```sh
   make check && make test
   ```
   or in `client/`: `npm run check && npm run lint && npm run test:unit -- --run && npm run build`.
   The coverage unit test confirms `es` has exactly `en`'s keys;
   `leetify:check` confirms `l33t.json` is current.

That's it — no Svelte/TS edits. `LanguageSwitcher` picks up the new locale and
its name automatically; `chrome.ts` localizes adapter chrome from the new keys.

## Recipe: add a content language (keyboarding word set)

This is independent of viewer languages — a word set needs no matching UI
language, and vice versa. To add e.g. a Spanish word set:

1. In `adapters/keyboarding/src/lib.rs`:
   - Add a word list, e.g. `const WORDS_ES: &[&str] = &[ ... ];`.
   - Add a `SelectChoice { value: "spanish", label: "Spanish" }` to the `wordSet`
     select in `option_schema()`.
   - Add a match arm in `next_prompt`:
     `Some("spanish") => WORDS_ES[(seed as usize) % WORDS_ES.len()].to_string(),`.
   - Add a unit test (mirror `l33t_word_set_leetifies_the_prompt` /
     `option_schema_exposes_word_set`).
2. So Spanish-viewing hosts see the option translated, add the chrome key to the
   catalogs: `choice_keyboarding_wordSet_spanish` in `client/messages/en.json`
   (then `npm run leetify`) and any other catalogs. Omitting it is safe — it just
   falls back to the English label `"Spanish"`.
3. Verify: `cargo test -p keyboarding` (or `make test`).

## Invariants & gotchas

- **Never hand-edit `messages/l33t.json` or `src/lib/paraglide/`** — both are
  generated. Edit `en.json` then run `npm run leetify` / `npm run paraglide:compile`.
- **Key parity is mandatory.** Every catalog has the same keys as `en.json`
  (enforced by `coverage.test.ts` and, for l33t, `leetify:check`).
- **Keep `$schema` as the first key** of every catalog.
- **SSR / first paint is always English**, then the client re-renders in the
  stored locale (strategy `['localStorage', 'baseLocale']`). The switcher
  reloads the page on change. Do **not** "fix" this by adding URL-based locale
  routing.
- **Locale codes are lowercase tokens** (`en`, `l33t`, `es`). If you ever need a
  region tag, note message keys can't contain hyphens — `chrome.ts` and
  `LanguageSwitcher` map `-` → `_` for lookups, so the catalog key for `pt-BR`
  would be `lang_name_pt_BR`.
- **Adapter chrome degrades to English** for any missing key, so a partially
  translated adapter is safe — but parity means you must still include the keys.
