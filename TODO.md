# TODO / Handoff

## Status
- **082.1 (pcap-aligned, no speculation):**
  1. `ptsSend` was wrongly set equal to `pts` — dumpsmall shows small latency
     (0..~6700). Fixed.
  2. Removed one-shot connect bursts that sent `ptsSensor=0` and leftover
     `pts=16666` (StartSensor echo + ctrl-caps). Those contradict dumpsmall
     (frame 0 already has a real pose time) and stall Present.
  3. Continuous path only: `pts = frameNum * 166666`, `ptsSensor = pose time`,
     `ptsServerLat ≈ stream age µs`, `ptsSend/ptsEncoderLat = 0` until measured.
- Still open: full pts clock model (Windows changes rate mid-session); whether
  zero encoder lat is accepted; Present/sensor match under high lag.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```
No one-shot "VideoFrame live IDR" logs at connect — only continuous `live frame #…`.

## Bundle
`/home/workdir/artifacts/relivevr-linux-082.1-pcap-ptssend-no-burst-f993e2b.bundle`
