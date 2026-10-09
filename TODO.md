# TODO / Handoff

## Status
- Video + pose + Daydream trackpad confirmed live.
- **OpenGL visualizer** (`RELIVEVR_VIZ=1`): grid, HMD/controller wire boxes with
  local axes, trackpad gizmo on the controller. Shares `LatestPoses` via
  `std::sync::Mutex` (pose types in `src/pose.rs`).

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# Window: "ReliveVR pose visualizer" — move headset / trackpad
```

## Next
1. Realtime encode: render viz (or app FBO) → H.264 → existing video channel
2. OpenXR / monado driver stub reading `LatestPoses`
3. Optional HEVC
4. ALVR/WiVRn evaluation

## Bundle
Apply: `git pull /path/to/relivevr-linux-038.1-opengl-viz-6813f93.bundle HEAD`
Tip: (after commit)
`/home/workdir/artifacts/relivevr-linux-038.1-opengl-viz-6813f93.bundle`

## Notes
- Base: 6813f93
