# TODO / Handoff

## Status
- Video + pose + trackpad working.
- OpenGL viz (`RELIVEVR_VIZ=1`): **main thread** runs winit EventLoop;
  UDP server runs on a background tokio runtime (no `any_thread`).

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# Expect window "ReliveVR pose visualizer"
```

## Next
1. Realtime encode: FBO → H.264 → video channel
2. OpenXR / monado stub

## Bundle
Apply: `git pull /path/to/relivevr-linux-043.1-viz-main-thread-6813f93.bundle HEAD`
`/home/workdir/artifacts/relivevr-linux-043.1-viz-main-thread-6813f93.bundle`

## Notes
- Base: 6813f93
