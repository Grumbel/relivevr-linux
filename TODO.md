# TODO / Handoff

## Current tip
- Based on github.com/Grumbel/relivevr-linux (0c00220 + early VideoInit).
- Connect works; StartRequest handled; VideoInit sent (type default 2).
- Early VideoInit after HELLO_DIRECT (decoder fails ~30ms after connect with video/).

## Test
```bash
nix run .
RELIVEVR_VIDEOINIT_TYPE=2 nix run .
RELIVEVR_VIDEOINIT_TYPE=4 nix run .
RELIVEVR_VIDEOINIT_TYPE=8 nix run .
adb logcat -s AMF_TRACE:D | grep -iE 'Video|Codec|Decoder|Init|Error|Connect'
```

## Next
1. Confirm VideoInit type/channel so MIME becomes video/avc
2. Binary H.264 SPS/PPS + IDR
3. Session keepalive (10s rediscovery)
