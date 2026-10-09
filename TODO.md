# TODO / Handoff

## Current tip
- StreamFlowCtrl 7-byte channel header implemented for VideoInit trials.
- Bundle stacks on github.com/Grumbel/relivevr-linux.

## Test
```bash
nix run .
# After StartRequest should log both:
#   VideoInit plain type=…
#   VideoInit stream ch=… type=…
adb logcat -s AMF_TRACE:D | grep -iE 'Video|Codec|Decoder|avc|CHANNEL|Error'
```

## Next
1. Identify which VideoInit form client accepts (logcat change)
2. Binary H.264 on video channel
3. Keepalive
