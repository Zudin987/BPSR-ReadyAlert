# BPSR Ready Alert 1.0 Release Checklist

Use this checklist on the final release-candidate EXE before creating the `v1.0.0` tag.

## Build / packaging

- [ ] Pull-request CI is green.
- [ ] `BPSR-ReadyAlert.exe` passes the bundle smoke test.
- [ ] EXE is below the 55 MiB CI size budget.
- [ ] `SHA256SUMS.txt` is present and matches the EXE.
- [ ] Explorer shows the custom application icon.
- [ ] System tray shows the custom application icon.
- [ ] No WinDivert files are bundled or created.
- [ ] Npcap remains an external dependency only.

## Clean-machine / startup behavior

- [ ] With Npcap installed, ReadyAlert starts without an Administrator/UAC prompt on a normal Npcap install.
- [ ] First run finds Resonance Logs CN automatically, or asks for its EXE once if not found.
- [ ] Auto-launch starts Resonance Logs CN when it is not running.
- [ ] Auto-launch does not start a duplicate when Resonance Logs CN is already running.
- [ ] Starting a second ReadyAlert instance reports that the tray app is already running.
- [ ] Start Menu shortcut opens the current ReadyAlert EXE and shows the custom icon.

## Npcap / adapter behavior

- [ ] Only one Npcap adapter is opened.
- [ ] Default selection follows Resonance Logs CN when its saved Npcap adapter is available.
- [ ] Manual Network Adapter selection is saved across restarts.
- [ ] Switching adapters restarts capture successfully.
- [ ] Follow Resonance Logs CN / Auto removes the manual override.
- [ ] A stale/unavailable saved adapter falls back safely and the stale override is cleared.
- [ ] ReadyAlert and Resonance Logs CN can capture the same adapter simultaneously.

## Alerts

- [ ] Test Alert Sound is clean, not robotic/glitchy.
- [ ] Alert Volume works at 100%, 50%, 10%, and Mute without changing Windows master volume.
- [ ] Real Ready Check plays the alert promptly.
- [ ] Real matchmaking accept/queue popup plays the alert promptly.
- [ ] Party/dungeon voting popup plays the alert when available to test.
- [ ] Duplicate packets do not cause rapid repeated alert spam.
- [ ] Disabling Queue Pop Alert suppresses only queue alerts.
- [ ] Disabling Ready Check Alert suppresses only Ready Check alerts.
- [ ] Desktop Notification toggle behaves independently from sound alerts.

## Stability / diagnostics

- [ ] Leave ReadyAlert running with BPSR for at least one normal play session without a crash or capture stall.
- [ ] Changing scenes / matchmaking connections does not permanently stop detection.
- [ ] `readyalert.log` records startup, adapter, capture and alert diagnostics.
- [ ] Log rotation prevents the active log from growing indefinitely beyond roughly 2 MiB.
- [ ] Exiting from the tray stops capture and closes cleanly.

## Release

- [ ] Update project version to `1.0.0`.
- [ ] Update README if any behavior changed during RC testing.
- [ ] Merge the audited PR into `main` only after live testing.
- [ ] Confirm the `main` Latest Build succeeds after merge.
- [ ] Create/push tag `v1.0.0` only after the merged build is verified.
- [ ] Verify the versioned GitHub Release contains `BPSR-ReadyAlert.exe` and `SHA256SUMS.txt`.
