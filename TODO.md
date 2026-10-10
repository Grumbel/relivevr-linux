# TODO / Handoff

## Status
- **081.1:** Drop pose/IDR stream-arm hold (regressed openh264 → black). Send
  immediately like Windows. Keep `pts = frameNum * 166666`, `ptsSensor` = latest
  pose `time` or 0. Windows pcap: every ptsSensor is an exact pose time; frameNum
  ≠ pose frmIdx.
- Present `pts=N not found` + sensor miss still open under lag; picture path
  should work again without the hold.
- Crop: prefer native 1440 for Daydream.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-081.1-no-hold-windows-pts-f993e2b.bundle`
