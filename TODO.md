# TODO / Handoff

## Status
- Video + pose + button/axis DeviceEvents working.
- **Bugfix:** channel-4 DeviceEvents were hex-dumped and skipped before JSON
  parse; pose/input now parsed on the pose channel (flags/ch=4).
- Per-packet `from …` log suppressed for pose channel; use `POSE #N` / `INPUT`.

## Test
```bash
cargo run
# After connect: POSE #1 … then every ~2s pose state
# Daydream button → INPUT sys_click #N
```

## Next
1. OpenXR / monado driver stub reading `LatestPoses`
2. Confirm live volume/trackpad path shapes against a real session
3. Optional HEVC
4. ALVR/WiVRn evaluation
5. Runtime image → H.264 (x264 / ffmpeg) instead of baked IDR patterns

## Bundle
Apply: `git pull /path/to/relivevr-linux-036.1-parse-pose-channel-6813f93.bundle HEAD`
Tip: 1554220
`/home/workdir/artifacts/relivevr-linux-036.1-parse-pose-channel-6813f93.bundle`

## Notes
- Base: 6813f93
