# TODO / Handoff

## Status (2026-10-09 evening)

### Video — solid
- Dual-eye H.264 1440×1440, LEFT/RIGHT grid patterns
- Decoder lag ~8–11 ms
- type 9 quiet when stream is fed
- ReInit(avc) confirmed in logcat

### Pose — blocked
- No channel-7 / sensor packets in any session
- Caps ACK + ch7 DeviceEvent probe (tip 013): **no effect**
- Client still only sends: Hello, caps, StartRequest, type 9

### Hypotheses (remaining)
1. `SendSensorData` gated on Windows OpenVR driver behaviour / feature flag
2. Needs TCP or a service opcode we have not discovered
3. Static RE of `libwirelessvr-lib.so` required to see call conditions

## Test
```bash
nix run .
# video only; no pose expected until RE finds enable path
```

## Next
1. Obtain APK (`com.amd.wirelessgvr` 1.0.13) + `libwirelessvr-lib.so`
2. Static RE: xrefs to `SendSensorData` / `SensorEngine` / `SendControllerData`
3. Optional: Wireshark capture against official Windows ReliveVR
4. Clean VideoInit spray; then monado/ALVR path

## Bundle tip
See last commit / artifacts for 013.1

## Notes
- Base: 6813f93

## Bundle
`/home/workdir/artifacts/relivevr-linux-014.1-caps-ack-no-pose-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-014.1-caps-ack-no-pose-6813f93.bundle HEAD`
Tip: 6bbce9d
