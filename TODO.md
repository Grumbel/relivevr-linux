# TODO / Handoff

## Status
- **080.1:** Windows dumpsmall.pcapng decoded: `pts = frameNum * 166666`
  (not 16666), `ptsSensor` = client pose `time` (~1e16), already set on frame 0.
  IDR every 120 frames (`encType` 0). Stream still arms on first pose + IDR so
  MediaCodec is not fed P-only.
- Present `pts=0 not found` at start also appears on Windows when pts=0; sustained
  queue growth + green was wrong tick / missing IDR.
- Crop: prefer native 1440 for Daydream.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# CPU-friendly:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```
Expect server `stream armed: pose_base=… first IDR idx=…`. Logcat may still show
brief `Present pts=0` (Windows does too); should not stay green or fill decoder.

## Bundle
`/home/workdir/artifacts/relivevr-linux-080.1-windows-pts-tick-f993e2b.bundle`
