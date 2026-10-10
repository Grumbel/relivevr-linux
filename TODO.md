# TODO / Handoff

## Status
- Tip: `/home/workdir/artifacts/relivevr-linux-074.1-fix-compile-hevc-f993e2b.bundle`
- Compile fixes: `EncoderKind::Hevc`, `codec_for_init`, `gop_s.clone()`.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=hevc RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```
