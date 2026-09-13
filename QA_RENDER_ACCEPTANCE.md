# ReadyAlert rendered UI acceptance

This follow-up closes the remaining evidence gap from the strict UI QA handoff by turning the live Meter DPI review into a reproducible Windows render matrix.

The native renderer is exercised at 100%, 125%, 150%, 175%, and 200% scale across Normal, Compact, Raid, and Raid + Compact modes, with both narrow/minimum and wide widths. The generated BMP diagnostics use the same transparent text/DC setup and DPI mapping path as the production overlay.

This is validation-only. It does not alter capture, telemetry, encounter logic, shortcuts, layouts, command IDs, or the native Rust/Win32 architecture.
