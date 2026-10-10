# TODO / Handoff

## Status (tip f993e2b)
- Stereo HMD-camera live encode (IPD 64 mm, dual OpenH264) working.
- Encode resolution **configurable**: default **720×720**, target **~75 Hz** (client native 1440×1440 / ~74.8 fps). Override with `RELIVEVR_ENCODE_W/H` and `RELIVEVR_ENCODE_FPS`.
- FOV adjustable live via Daydream **vol+/-** (40–120°, step 5°); default 70° vertical; near 0.08 m.
- Checkerboard room (floor + ceiling + four walls, ±2.5 m).
- Window title shows FOV; parallel CPU encode of the two eyes.
- Pose, trackpad, menu/vol/sys/app buttons visualized.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# optional: RELIVEVR_ENCODE_W=480 RELIVEVR_ENCODE_H=480 RELIVEVR_ENCODE_FPS=30
```

## Next
1. Further FOV / distortion match to Daydream lenses (Hello advertises ~100° H/V FOV; currently plain perspective, no radial distortion mesh/shader).
2. Optional higher res / GPU encode when CPU allows (or lower default for smoother 75 Hz).
3. OpenXR / monado stub.

## Bundle
(previous: `relivevr-linux-051.1-perf-fov-tune-6813f93.bundle` — superseded by later tip commits; new tip bundle to be produced after next work)
