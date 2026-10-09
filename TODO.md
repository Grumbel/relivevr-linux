# TODO / Handoff

## Status — POSE WORKING
- Server StartSensor (type 5 S→C) after VideoInit unlocks pose.
- Client streams `/hmd/pose` + `/ctrlRight/pose` (orient, pos, battery).
- Video dual-eye patterns still working.
- Pose log rate-limited (first 3 + every 120).

## Test
```bash
cargo run   # or nix run .
# Expect: StartSensor S→C → POSE #0 … with orient/pos
```

## Next
1. Structured pose parse (serde) → expose for OpenXR/monado later
2. Controller button paths beyond pose (inputs in caps)
3. Clean dead code warnings (CHANNEL_SERVICE, stream helpers)
4. Optional: HEVC path (Windows uses hevc; we use avc successfully)

## Notes
- Base: 6813f93
- Key captures: dump.pcapng, cap2.pcapng

## Bundle
`/home/workdir/artifacts/relivevr-linux-032.1-pose-working-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-032.1-pose-working-6813f93.bundle HEAD`
Tip: 50cba6f
