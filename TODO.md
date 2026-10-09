# TODO / Handoff

## Status
- Stereo HMD-camera live encode works but was slow / artifacty.
- Tuned: **512×512**, ~20 fps cap, 4 Mbps/eye, FOV **70°**, near 0.08.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
```

## Next
1. Further FOV / distortion match to Daydream lenses
2. Optional higher res when CPU allows
3. OpenXR stub

## Bundle
`/home/workdir/artifacts/relivevr-linux-051.1-perf-fov-tune-6813f93.bundle`
