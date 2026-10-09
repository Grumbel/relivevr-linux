# TODO / Handoff

## Status
- Video + pose + Daydream trackpad (val/touch/click) confirmed live.
- 2D trackpad axes stored; continuous INPUT rate-limited; type-6 FrameRate handled.
- Channel-4 DeviceEvent JSON parse fixed (036.1).

## Test
```bash
cargo run
# POSE #N … with in=[in/tp/touch,in/tp/val=[x,y],in/tp/click]
# click/touch edges → INPUT …; axis motion sparsely logged
```

## Next
1. OpenXR / monado driver stub reading `LatestPoses`
2. Volume-button paths when observed live
3. Optional HEVC
4. ALVR/WiVRn evaluation
5. Runtime image → H.264 (x264 / ffmpeg)

## Bundle
Apply: `git pull /path/to/relivevr-linux-037.1-trackpad-2d-6813f93.bundle HEAD`
Tip: bf33088
`/home/workdir/artifacts/relivevr-linux-037.1-trackpad-2d-6813f93.bundle`

## Notes
- Base: 6813f93
