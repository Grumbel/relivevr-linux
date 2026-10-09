# TODO / Handoff

## Status
- Video + pose + trackpad working.
- OpenGL viz (`RELIVEVR_VIZ=1`) uses `EventLoopBuilderExtUnix::any_thread`
  so the window can run off the tokio main thread.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# Window should open; move headset / trackpad
```

## Next
1. Realtime encode: FBO → H.264 → video channel
2. OpenXR / monado stub
3. Optional HEVC
4. ALVR/WiVRn evaluation

## Bundle
Apply: `git pull /path/to/relivevr-linux-039.1-viz-any-thread-6813f93.bundle HEAD`
Tip: 0dbadec
`/home/workdir/artifacts/relivevr-linux-039.1-viz-any-thread-6813f93.bundle`

## Notes
- Base: 6813f93
