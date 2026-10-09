# TODO / Handoff

## Status
- Video solid (~10ms lag, dual-eye patterns).
- Pose blocked: SensorThread never active.
- Empty type-4 `{}` = Daydream button (not CStartSensor, not pose).
- StartCommunications on client builds StartRequest (not a missing server msg).
- Extra opcodes documented: VideoForceIDR, UpdateRequest, StopRequest, ProfileNetwork*.

## Test
```bash
nix run .
# Daydream button → type-4 empty {}
# No orient/pos expected until SensorThread activation found
```

## Next (highest value)
1. **Windows ReliveVR + Wireshark** on UDP 1235 after connect — ground-truth
   service messages while pose is streaming
2. **Frida** on-device: hook `SensorThread::SetActive` / `QueryAndSendSensors`
3. Obtain Daydream 1.0.13 `libwirelessvr-lib.so` for side-by-side vs Oculus SO
4. Clean VideoInit spray; optional ProfileNetwork handling

## Notes
- Base: 6813f93
- Analyzed: GPUOpen Oculus 1.0.26 + ReLive 2.0 beta APKs (not committed)
