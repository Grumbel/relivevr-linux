# TODO / Handoff

## Status
- **089.1:** Wait for ≥30 `/hmd/pose` samples before first VideoData (dumpsmall
  has ~43 HMD poses before first ptsSensor). Warm client pose queue for APK
  exact-match Present lookup.
- Prior: HMD-only ptsSensor, freshest at send, diagnostics.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
```
Server should log `waiting for HMD pose queue warm-up` then `live frame #0`
with `hmd_poses>=30`.

## Bundle
`/home/workdir/artifacts/relivevr-linux-089.1-pose-queue-warmup-f993e2b.bundle`
