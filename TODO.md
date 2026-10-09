# TODO / Handoff

## Milestone
- **Frames reach decoder** (`Decoder lag around frame #3`)
- Real 1440×1440 H.264 IDR embedded
- Continuous ~60 fps stream after StartRequest
- Type 9 → force IDR

## Test
```bash
nix run .
# Expect: VideoFrame IDR ~6-7KB (not 170B)
adb logcat -s AMF_TRACE:D | grep -iE 'Frame|lag|Submit|Decoder|Reset|Error'
```

## Next
1. Confirm blue image on headset with real IDR + continuous stream
2. AVCC vs Annex-B if decode errors appear
3. Pose / controllers
