# TODO / Handoff

## Status
- Video + pose + button/axis DeviceEvents working.
- `LatestPoses` holds HMD/controller orient/pos, batteries, `inputs` map
  (path → pressed/axis), and `system_clicks` (empty `{}` type-4).
- Logs: `INPUT …` on non-pose DeviceEvents; periodic `pose state #N …`.

## Test
```bash
cargo run
# POSE #1 … then every ~2s: pose state #N /hmd q=[…] | /ctrlRight q=[…]
# Press Daydream button → INPUT sys_click #N
# Volume / trackpad (when emitted) → INPUT /ctrlRight/in/…
```

## Next
1. OpenXR / monado driver stub reading `LatestPoses`
2. Confirm live volume/trackpad path shapes against a real session
3. Optional HEVC
4. ALVR/WiVRn evaluation
5. Runtime image → H.264 (x264 / ffmpeg) instead of baked IDR patterns

## Bundle
Apply: `git pull /path/to/relivevr-linux-035.1-input-events-6813f93.bundle HEAD`
Tip: ae057d9
`/home/workdir/artifacts/relivevr-linux-035.1-input-events-6813f93.bundle`

## Notes
- Base: 6813f93
