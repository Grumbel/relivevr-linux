# TODO / Handoff

## Status
- **087.1:** `ptsSensor` at send = freshest `/hmd/pose` `latest_time` (fallback:
  render stamp). APK does exact pose-queue match; render-time stamps can age out
  under 80–300 ms lag. Still HMD-only (085.1).
- Docs: `docs/client-apk.md`, `docs/windows-pcap.md`.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
```
Log: `ptsSensor=` non-zero; `sensor pts not found` / `prev=0` should clear.

## Bundle
`/home/workdir/artifacts/relivevr-linux-087.1-fresh-hmd-ptssensor-f993e2b.bundle`
