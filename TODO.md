# TODO / Handoff

## Status
- OpenH264: live picture on Daydream (dual encoder).
- HEVC path: `RELIVEVR_ENCODER=hevc` (hevc_nvenc → hevc_vaapi → libx265).
- README / AGENTS / re-notes updated for encoder matrix, dual-encoder, large UDP, HEVC.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=hevc \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Next
- Validate HEVC on headset; tune initial IDR / garble.
- Wire SoftEncoder `force_idr` for faster recovery.

## Bundle
`/home/workdir/artifacts/relivevr-linux-073.1-docs-encoder-hevc-f993e2b.bundle`
