# Suggested Commit Series

This is a clean, phase-oriented commit plan for the current rewrite.

## 1) `refactor(startup): harden boot flow and safe-mode fallback`

Scope:
- `js/index.js`
- `js/scene.js`
- `js/audio.js`
- `js/events.js`

Focus:
- Timeout-guarded startup steps
- Non-blocking optional systems
- Degraded state signaling and manual start fallback

## 2) `fix(terminal): responsive canvas, wrapped input, and scroll/history behavior`

Scope:
- `src/terminal/core.rs`
- `src/terminal/renderer.rs`
- `src/terminal/buffer.rs`
- `src/input/history.rs`
- `src/input/setup.rs`

Focus:
- DPI-aware sizing and alignment
- Wrapped input with cursor correctness
- Better scroll controls and draft-preserving history

## 3) `refactor(shell): add parser module with chain, pipe, and redirection`

Scope:
- `src/shell/mod.rs`
- `src/commands/processor.rs`
- `src/commands/commands.rs`
- `src/commands/options.rs`

Focus:
- Extract parser boundary (`src/shell`)
- Standardized option parsing and error output
- Basic pipeline and redirection emulation

## 4) `feat(github): live project/readme fetch with cache metadata`

Scope:
- `src/github.rs`
- `src/commands/processor.rs`
- `src/commands/registry.rs`

Focus:
- Live GitHub data only
- Cached/live/stale-cache source metadata
- `projects`, `project`, `readme`, `open`, `refresh`

## 5) `feat(guide): add config-driven onboarding panel`

Scope:
- `static/index.html`
- `static/styles.css`
- `js/index.js`
- `js/guide.js`
- `static/guide.json`

Focus:
- GUI guidance panel (not terminal command)
- Click-to-fill and click-to-run actions
- Mobile-safe onboarding layout

## 6) `chore(content): externalize runtime text and remove placeholders`

Scope:
- `src/boot/boot.rs`
- `src/commands/filesystem.rs`
- `static/content/boot.json`
- `static/content/filesystem.json`
- `docs/VALIDATION_NOTES.md`
- `REFACTOR_TRACKER.md`

Focus:
- Move boot/filesystem content to JSON
- Remove static fake project entries
- Keep validation and tracker docs in sync
