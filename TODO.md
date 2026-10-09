# TODO / Handoff

## Current tip
- On upstream github.com/Grumbel/relivevr-linux + early VideoInit + multi-type probe.
- After StartRequest, probe sprays VideoInit types 2/4/8/9/10 × codec variants.

## Watch
```bash
nix run .
adb logcat -s AMF_TRACE:D | grep -iE 'Video|Codec|Decoder|avc|Init|Error'
```
Hope: MIME becomes video/avc or decoder starts.

## Next
1. Narrow VideoInit type from logcat / successful decode
2. Binary H.264 framing (channel + NAL)
3. Keepalive
