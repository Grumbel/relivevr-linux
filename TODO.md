# TODO / Handoff

## Status
- Code review fixes:
  1. **Seed encode** after H264Encoder create → SPS/PPS in live slot before clients.
  2. **VideoInit** never falls back to baked-pattern SPS when live encoder is on
     (waits up to ~1s for param sets).
  3. **publish_stereo** captures SPS/PPS from first AU that has them (not only IDR).
- FFmpeg pipe: 4MiB buffers, poll reader, one-frame warm-up (prior tips).

## Known remaining
- `force_idr` is computed but ignored by SoftEncoder and FFmpeg backends (GOP only).
- Parallel dual FFmpeg at 1440² still needs runtime confirmation of warm-up OK.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: live video SPS/PPS …B  before any client connect
#         VideoInit 720x720 codec param sets …B  (not 38B baked)

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect: warm-up OK, then live SPS/PPS, live IDR frames
```

## Next
1. Confirm live scene on HMD.
2. Wire force_idr for OpenH264/FFmpeg if reconnect needs mid-stream IDR.
3. Radial distortion / OpenXR.

## Bundle
`/home/workdir/artifacts/relivevr-linux-061.1-seed-sps-videoinit-f993e2b.bundle`
