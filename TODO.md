# TODO / Handoff

## Status
- Video solid (~10ms lag, dual-eye patterns).
- **Removed** caps ACK + ch7 DeviceEvent probe (tip 013–014): likely caused
  native client crash/restart (new PID in logcat; Init then StartDiscovery).
- Never reply with guessed JSON for opcodes whose FromJSON is strict.

## Test
```bash
nix run .
# Expect: stable session, no app restart; LEFT/RIGHT grids
```

## Next
1. Confirm client stays up (same PID) for minutes
2. Pose: needs static RE of SendSensorData in libwirelessvr-lib.so
3. Clean VideoInit spray
4. monado / ALVR later

## Notes
- Base: 6813f93
