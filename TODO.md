# TODO / Handoff

## Status (pcap correction)
Windows uses **single large UDP datagrams** (up to ~65KB, IP-fragmented),
not 1400B FlowCtrl splits. Our 1400B split was wrong → green.

Also: SoftEncoder now seeds SPS; openh264 black may have been empty
param_sets / no seed.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```
Expect non-green/non-black; logcat without mass "Message is old".

## Next
HEVC path if AVC still fails (Windows native is hevc).

## Bundle
(to be produced)
