# Rust migration status

This directory contains the native Windows Rust rewrite of BPSR Ready Alert.

The existing C# v1.3.6 implementation remains in `src/BPSR.ReadyAlert` as a validated fallback while the Rust implementation is compiled and exercised by Windows CI. Do not switch the stable release workflow to Rust until the native workflow passes and the resulting executable is live-tested against BPSR/Npcap.

The Rust implementation preserves the existing `%LOCALAPPDATA%\\BPSR-ReadyAlert` settings/log location and intentionally keeps the capture, protocol, chat, TTS, alert, and recovery behavior in one native process.
