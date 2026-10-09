# TODO / Handoff

## Status
- Pcap: pose after client `StartSensor` (type 5), then type-4 orient/pos.
- Hello/VideoInit aligned with Windows.
- **Fixed HelloResponse loop**: ignore ChannelsSupported / port 1235; detect own
  ServerName; announce off unless `RELIVEVR_ANNOUNCE=1`.

## Test
```bash
nix run .
# Stop other ReliveVR servers on the LAN
# Headset discovery → expect client DeviceID Hello, not probe loop
# Then StartSensor + POSE lines
```

## Next
1. Confirm StartSensor + pose with headset only
2. Optional HEVC NALs after VideoInit
3. Structured pose parse

## Notes
- Base: 6813f93

## Bundle
`/home/workdir/artifacts/relivevr-linux-023.1-fix-hello-loop-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-023.1-fix-hello-loop-6813f93.bundle HEAD`
Tip: a42a6fd
