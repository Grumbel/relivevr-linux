# TODO / Handoff

## Status
- VideoInit after ctrl caps (pcap order) works.
- Still no client StartSensor.
- Tip 026: VideoInit body = `0x00` + JSON + H.264 SPS/PPS (matches Windows layout).

## Test
```bash
nix run .
# Expect: VideoInit codec param sets …B, then hopefully StartSensor
```

## Next
1. Confirm StartSensor
2. If not: capture our server vs Windows side-by-side with Wireshark
3. Pose parse

## Notes
- Base: 6813f93

## Bundle
`/home/workdir/artifacts/relivevr-linux-026.1-videoinit-sps-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-026.1-videoinit-sps-6813f93.bundle HEAD`
Tip: d21f626
