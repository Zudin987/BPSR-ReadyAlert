# ReadyAlert rendered UI acceptance

This follow-up closes the remaining evidence gap from the strict UI QA handoff by turning the live Meter DPI review into a reproducible Windows render matrix.

The native renderer is exercised at 100%, 125%, 150%, 175%, and 200% scale across Normal, Compact, Raid, and Raid + Compact modes, with both narrow/minimum and wide widths. The generated BMP diagnostics use the same transparent text/DC setup and DPI mapping path as the production overlay.

## Acceptance result

The complete 40-image matrix was inspected after the Windows CI run. The rendered Meter passed the acceptance pass at every requested scale and width combination:

- no toolbar overlap or left-edge overflow
- Live, Raid and Compact semantic selection stays on the correct action
- previous/next controls do not inherit a neighboring selected state
- long/localized player and specialization text truncates intentionally instead of drawing outside its column
- Compact rows remain visibly thinner than normal rows
- `CAN REVIVE` remains readable in Compact and Raid + Compact layouts
- Raid keeps ranks 1-10 and 11-20 in separate columns at narrow and wide widths
- row fills, class tinting, selection outline, progress treatment and center gutter remain intact through 200%
- the diagnostic renderer now mirrors production transparent text rendering instead of producing false opaque-white text boxes

The full native Rust CI pipeline also passes with the matrix enabled: unit/regression tests, Clippy, release build, EXE smoke test, size budget, generated-source diagnostics and native render diagnostics.

## Empty-state coverage

The generated-source QA gate now permanently verifies the structured states that were called out in the strict audit: Chat waiting/no-messages, DPS waiting-for-combat, Raid waiting-for-data, Dungeon Mechanics no-active-mechanics, Settings Blocked Users/custom tabs, and Event Tracker rules. Encounter History keeps its own structured browser-side empty state and tests.

This is validation-focused. It does not alter capture, telemetry, encounter logic, shortcuts, layouts, command IDs, or the native Rust/Win32 architecture.
