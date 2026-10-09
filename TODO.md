# TODO / Handoff

## Status — POSE WORKING
- StartSensor S→C after VideoInit → pose stream.
- Structured `DeviceEventMsg` parse + compact log (`q=` / `p=`).
- Video dual-eye OK.

## Test
```bash
cargo run
# POSE #0 ch=4 /hmd/pose q=[…] p=[…] | /ctrlRight/pose q=[…] p=[…]
```

## Next
1. Shared latest-pose state (Arc) for OpenXR/monado consumer
2. Controller button/axis events from caps inputs
3. HEVC optional path
4. ALVR / WiVRn integration evaluation

## Bundle
`/home/workdir/artifacts/relivevr-linux-033.1-pose-parse-6813f93.bundle`

## Notes
- Base: 6813f93
