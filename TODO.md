# TODO / Handoff

## Status
- **086.1:** Documented client APK RE (`docs/client-apk.md`):
  Present does `pts → ptsSensor` (exact), then pose queue by sensor time
  (exact). Confirms 085.1: ptsSensor must be `/hmd/pose` time only.
- Code tip remains **085.1** (HMD-only latest_time).

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-086.1-docs-client-apk-f993e2b.bundle`
