# TODO / Handoff

## Status
- **079.1:** Stream arm requires first client pose **and** an IDR AU before any
  video is sent. Holding for pose alone skipped the encoder’s initial IDR →
  decoder saw only P-frames → solid green. Synthetic PTS timeline:
  `pts = pose_base + frameNum * 13_333_333` (ns, ~75 Hz); `ptsSensor` = latest
  pose `time` when available.
- Present log `pts=N` is the frame index (frameNum), not the JSON pts value.
- Sensor pts exact-match still may fail under high lag (pose aged out); synthetic
  timeline + IDR start is the main green fix.
- Crop: prefer encode at native 1440 for Daydream.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# or CPU-friendly:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```
Watch server log for `stream armed: pose_base=… first IDR idx=…` then live frames.
Logcat should stop solid-green if the IDR-start was the cause; Present/sensor
warnings may linger until lag is lower.

## Bundle
`/home/workdir/artifacts/relivevr-linux-079.1-idr-on-arm-f993e2b.bundle`
