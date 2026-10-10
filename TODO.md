# TODO / Handoff

## Status (tip after FOV default bump)
- Stereo HMD-camera live encode (IPD 64 mm, dual OpenH264) working.
- Encode resolution **configurable**: default **720×720**, target **~75 Hz** (client native 1440×1440 / ~74.8 fps). Override with `RELIVEVR_ENCODE_W/H` and `RELIVEVR_ENCODE_FPS`.
- FOV adjustable live via Daydream **vol+/-** (40–120°, step 5°); **default now 90°** (was 70°); near 0.08 m.
  HelloResponse advertises ~100°; measured Daydream View ~89°. No radial distortion yet (see docs/re-notes.md).
- Checkerboard room (floor + ceiling + four walls, ±2.5 m).
- Window title shows FOV; parallel CPU encode of the two eyes.
- Pose, trackpad, menu/vol/sys/app buttons visualized.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# optional: RELIVEVR_ENCODE_W=480 RELIVEVR_ENCODE_H=480 RELIVEVR_ENCODE_FPS=30
# vol+/- on Daydream controller changes FOV at runtime
```

## Next
1. Radial distortion (barrel pre-distort) matching Daydream lenses — coefficients in docs/re-notes.md; confirm whether official server sends pre-distorted frames.
2. Optional higher res / GPU encode when CPU allows.
3. OpenXR / monado stub.

## Bundle
`/home/workdir/artifacts/relivevr-linux-052.1-fov-default-90-f993e2b.bundle`
