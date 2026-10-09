# TODO / Handoff

## Status
- Live H.264 of OpenGL viz → headset.
- **Fix:** streamer only sends each encoded AU once (no P-frame replay).
- Skip-frames disabled; ~6 Mbps, IDR every 30 frames.
- Still mono (same picture both eyes).

## Test
```bash
nix develop
RELIVEVR_VIZ=1 cargo run
# Hold controller still — image should stay stable, not garble
```

## Next
1. Stereo cameras (per-eye view)
2. Higher res / GPU encode
3. OpenXR stub

## Bundle
`/home/workdir/artifacts/relivevr-linux-049.1-no-dup-pframes-6813f93.bundle`

## Notes
- Base: 6813f93
