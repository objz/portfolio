# Validation Notes

This document tracks reproducible checks for the portfolio rewrite.

## Historical Repro Cases

1. Loader stuck on "Initializing..."
   - Condition: startup waits on optional systems indefinitely.
   - Check: throttle CPU/network in browser devtools and reload.
   - Expected now: `START` appears or safe mode notice appears; app remains interactive.

2. Mobile performance artifacts (triangle corruption)
   - Condition: previous geometry decimation in perf mode.
   - Check: open on mobile viewport, enable low-power/perf mode.
   - Expected now: no corrupted black triangles; scene remains stable.

3. Long input line wrapping/cursor drift
   - Condition: long commands exceeding first line width.
   - Check: paste a long command and move cursor with arrows/Home/End.
   - Expected now: wrapped rendering and cursor position stay aligned.

4. History navigation destroys in-progress input
   - Condition: type partial command, press ArrowUp then ArrowDown.
   - Expected now: previous partial command is restored.

5. Non-shell-like chaining and parsing
   - Check: run `pwd; ls`, `cd projects && ls`, `echo "a b"`.
   - Expected now: chain operators and quoted tokens parse correctly.

6. GitHub project placeholders not real
   - Check: run `projects`, `project <repo>`, `readme <repo>`.
   - Expected now: data comes from live GitHub API with cache metadata.

## Runtime Checks (Manual Browser)

- Desktop: load app, focus terminal, complete guide panel steps.
- Mobile viewport: check layout of guide panel and no model artifacts.
- Keyboard-only: Tab completion, Arrow history, PageUp/PageDown scroll, Ctrl+L clear.
- GitHub commands: confirm `source: live/cache/stale cache` metadata appears.
