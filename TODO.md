# TODO / Handoff

## Status
- OpenGL viz on main thread; live **H.264 encode** of the scene (OpenH264,
  720×720) published to the UDP stream when `RELIVEVR_VIZ=1`.
- VideoInit uses 720×720 + live SPS/PPS when param sets are ready.
- Without viz, behaviour unchanged (baked LEFT/RIGHT patterns).

## Test
```bash
nix develop
RELIVEVR_VIZ=1 cargo run
# Connect headset — should show the 3D scene (grid/HMD/controller), not test grids
```

## Next
1. Tune bitrate / resolution / stereo (per-eye cameras)
2. OpenXR / monado stub
3. Optional HEVC / hardware encode

## Bundle
`/home/workdir/artifacts/relivevr-linux-046.1-live-h264-encode-6813f93.bundle`

## Notes
- Base: 6813f93
