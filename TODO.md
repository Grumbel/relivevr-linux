# TODO / Handoff

## Done
- Video path **healthy**: decoder lag ~8–11 ms (was 17–31 s).
- Type 9 quiet when stream is fed (count=4 at connect, not hundreds/s).
- Dual-eye LEFT/RIGHT grid patterns confirmed earlier.
- Channel demux ready; **no pose/DeviceEvent packets** in any capture yet.

## Live (2026-10-09 15:52)
- Connect → caps → StartRequest → ReInit(avc) 1440×1440 → stream
- type9 count stays at 4 after initial burst
- logcat lag ~7–11 ms only

## Test
```bash
nix run .
# Expect: LEFT/RIGHT grids, lag ~10ms, quiet logs, type9 summary ~4
```

## Next (pose)
Client never sends channel-7 / multi-byte sensor data in our sessions.
Hypotheses to test (static RE + experiment):
1. Pose gated on server message we do not send (caps ACK, “tracking start”, …)
2. Pose only when Windows OpenVR driver is up (client detects via protocol feature)
3. Pose on a different transport (TCP) or after audio channel opens
4. Daydream 3DoF pose folded into type-9 or another opcode we misread

Practical next steps:
1. Static RE: `SendSensorData` / `SensorEngine` call sites in `libwirelessvr-lib.so`
2. Optional: try TCP listener on 1235; try minimal caps ACK; try audio init
3. Capture with official Windows ReliveVR if available for ground-truth packets
4. Clean up VideoInit spray once pose path is understood

## Notes
- Base: 6813f93
