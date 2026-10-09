# TODO / Handoff

## Status
- **Stereo live H.264** from HMD pose (IPD 64 mm), dual OpenH264 encoders.
- Window preview also tracks HMD camera (~90° FOV).
- Each eye’s AU sent once; no P-frame replay.

## Test
```bash
nix develop
RELIVEVR_VIZ=1 cargo run
# Look around with headset — view should follow; slight stereo parallax
```

## Next
1. Tune FOV / near plane / world scale
2. Higher res / GPU encode
3. OpenXR stub

## Bundle
`/home/workdir/artifacts/relivevr-linux-050.1-stereo-hmd-cam-6813f93.bundle`

## Notes
- Base: 6813f93
