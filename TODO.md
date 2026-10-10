# TODO / Handoff

## Status
- **090.1:** Cap live video to ~60 fps (16 ms min interval) — logcat showed
  MediaCodec "input is full". On IDR, prepend SPS/PPS if the AU lacks them.
- Prior: HMD-only ptsSensor, freshest at send, pose warm-up ≥30, diagnostics.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
```
Expect warm-up log, then ~60 fps frames with non-zero ptsSensor and hmd_poses≥30.

## Bundle
`/home/workdir/artifacts/relivevr-linux-090.1-fps-throttle-idr-sps-f993e2b.bundle`
