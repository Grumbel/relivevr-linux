# TODO / Handoff

## Status
- Video + pose + trackpad + OpenGL viz working.
- Viz: `RELIVEVR_VIZ=1`, X11 `with_any_thread`, `EventLoopBuilder::build()?`.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# Window "ReliveVR pose visualizer"; move headset / trackpad
```

## Next
1. Realtime encode: FBO → H.264 → video channel
2. OpenXR / monado stub
3. Optional HEVC
4. ALVR/WiVRn evaluation

## Bundle
Apply: `git pull /path/to/relivevr-linux-042.1-viz-working-6813f93.bundle HEAD`
Tip: see bundle HEAD
`/home/workdir/artifacts/relivevr-linux-042.1-viz-working-6813f93.bundle`

## Notes
- Base: 6813f93
