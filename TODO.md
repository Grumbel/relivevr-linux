# TODO / Handoff

## Status
- Video solid without CStartSensor.
- CStartSensor → client type-4 JSON `{}` only; no orient/pos; rediscovery often follows.
- Daydream button: user sees a local reaction; no controller packets on the wire.
- SensorThread / QueryAndSendSensors never runs in our sessions.

## Test
```bash
nix run .                         # video only — check session stability + button vs type-4
RELIVEVR_START_SENSOR=1 nix run . # single delayed CStartSensor (optional)
```

When pressing Daydream app/trackpad/home, note whether a type-4 `{}` appears
and the exact time vs button press.

## Next
1. Correlate type-4 `{}` with button presses (no CStartSensor)
2. Disasm Motor::SensorThread::SetActive / QueryAndSendSensors callers
3. Windows ReliveVR capture if available
4. Clean VideoInit spray

## Notes
- Base: 6813f93
