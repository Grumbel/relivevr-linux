# TODO / Handoff

## Status
- Green root causes addressed:
  1. **Dual encoders → different SPS** vs one VideoInit → single shared encoder
  2. **Tiny 81B skip AUs** sent as video → filter AUs &lt; 200B
  3. Multi-fragment UDP + repeat-headers (prior tip)

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: live frame #N L=thousands R=thousands (not 81B)
# single encoder log line
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-068.1-single-encoder-no-tiny-f993e2b.bundle`
