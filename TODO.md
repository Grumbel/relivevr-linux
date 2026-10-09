# TODO / Handoff

## Current tip
- Commit `442e0c5` – Fragment header reverse-engineered + enhanced probe.
- Bundle: `/home/workdir/artifacts/relivevr-linux-002.1-fragment-header-6813f93.bundle`
  (base still 6813f93; full history included).

## Open work
1. Continue static RE:
   - `Command::ParseBuffer` and discovery reply construction
   - Channel enum values
   - Video message format / SPS-PPS handling
   - Pose serialization
2. Capture real traffic (Windows + headset) if available.
3. Implement discovery responder once Command layout is clearer.
4. Video injector (hard-coded H.264/HEVC) once channel + encapsulation known.
5. Keep docs in sync.

## Next concrete steps
- [x] Fragment header layout reverse-engineered (15 bytes, BE fields).
- [x] Probe now parses and logs header + payload preview.
- [ ] Disassemble / understand `Command::ParseBuffer` and discovery type-0 handling.
- [ ] Add ability to send crafted discovery replies.
- [ ] Look for channel constants / switch tables near OnMessageReceived.

## Bundle history
- `relivevr-linux-001.2-...` (superseded)
- `relivevr-linux-002.1-fragment-header-6813f93.bundle` (current tip)

## Notes for next agent
- APK at `/home/workdir/attachments/com.amd.wirelessgvr_...apk`
- Work under `/tmp/relivevr-linux`, copy only finished bundles to artifacts.
- Fragment header is the biggest win this round; the probe is now useful for live traffic analysis.
