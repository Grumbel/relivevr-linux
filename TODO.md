# TODO / Handoff

## Status
- Video + pose + trackpad working.
- OpenGL viz (`RELIVEVR_VIZ=1`): `EventLoopBuilderExtX11::with_any_thread`
  (winit 0.29 has no `platform::unix`).

## Test
```bash
RELIVEVR_VIZ=1 cargo run
```

## Next
1. Realtime encode: FBO → H.264 → video channel
2. OpenXR / monado stub
3. Optional HEVC
4. ALVR/WiVRn evaluation

## Bundle
Apply: `git pull /path/to/relivevr-linux-040.1-viz-x11-any-thread-6813f93.bundle HEAD`
Tip: see bundle HEAD
`/home/workdir/artifacts/relivevr-linux-040.1-viz-x11-any-thread-6813f93.bundle`

## Notes
- Base: 6813f93
