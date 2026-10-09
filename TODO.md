# TODO / Handoff

## Status
- Video + pose working.
- `LatestPoses` Arc holds HMD/controller orient/pos + batteries; updated every DeviceEvent.
- Periodic `pose state #N …` log every 2s while tracking.

## Test
```bash
cargo run
# POSE #1 … then every ~2s: pose state #N /hmd q=[…] | /ctrlRight q=[…]
```

## Next
1. OpenXR / monado driver stub reading `LatestPoses`
2. Button/axis events (caps list volume clicks)
3. Optional HEVC
4. ALVR/WiVRn evaluation

## Bundle
`/home/workdir/artifacts/relivevr-linux-034.1-latest-poses-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-034.1-latest-poses-6813f93.bundle HEAD`

## Notes
- Base: 6813f93
