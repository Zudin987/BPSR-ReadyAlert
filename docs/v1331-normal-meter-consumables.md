# v1.33.1 Normal Meter Consumables

- Reuse the existing per-player Food and Serum state already used by Raid Mode.
- Normal non-compact meter rows reserve a small right-side slot inside each row.
- Active Food is shown as `F`; active Serum is shown as `S` using the same Raid Mode expiry colors.
- If neither consumable is active, the slot remains empty.
- The old floating consumable popup is hidden to avoid duplicate indicators outside the normal meter.
- Raid Mode layout and its existing consumable gutter remain unchanged.
- Compact Mode remains unchanged.
- No combat, DPS, healing, tank, telemetry, or consumable detection logic changes.
