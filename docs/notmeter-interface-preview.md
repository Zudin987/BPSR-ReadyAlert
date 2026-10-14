# NotMeter-inspired ReadyAlert interface preview

This branch applies a consistent charcoal-and-sage design to the native combat
meter, settings, combat details, trackers, alerts and local history reports.
NotMeter's public interface screenshots were used as a visual reference; the
implementation is native ReadyAlert code and uses its existing BPSR assets.

The four meter layouts remain available. Class gauges use rounded capsules,
while measured player columns, Imagine badges, consumable indicators, revive
states, hover information and click regions remain attached to their existing
data. Combat calculations and packet capture are unchanged.

Open the meter's settings icon to try the layout buttons and live preview.
The **Visual theme** selector switches between **NotMeter** and **ReadyAlert
Classic** immediately and saves the choice. The sidebar opens combat helpers,
alerts, Chat & TTS, history, general settings and network settings. Settings
remain scrollable when the work area is too small for the complete window.

Local Encounter History and Chat Archive use the same colors and readable type.
Encounter History adds class-colored roster gauges and actual recorded DPS;
existing player inspection, filters, sorting and two-encounter comparison remain
available. Reports refresh their appearance when reopened from ReadyAlert.

## Validation

The ordinary Windows CI remains the release gate: all-target unit tests, Clippy,
native release build, smoke test and the 20 MiB size limit. Its native render
artifact also contains the existing layout/DPI matrix and new native meter,
settings and combat-details screenshots, plus a self-contained sample history
report. The additional screenshots and report contain illustrative QA data.

The generated-source bridge is in `rust/build/design.rs`; runtime UI and theme
logic live in ordinary checked-in source files. No generated `OUT_DIR` file is
edited or committed as the implementation.
