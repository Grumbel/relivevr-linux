# TODO / Handoff

## Done
- Type 9 is high-rate (100s/sec) 1-byte keepalive — **must not** answer with IDR.
  Previous tip's force-IDR on every type 9 flooded the decoder (lag ~17–31s).
- Type 9 now: count only, keep session armed, log summary every 2s announce tick.
- Stream rate set to ~60 fps (StartRequest FrameRate).
- PTS = wall-clock µs from stream origin.

## Test
```bash
nix run .
# Expect: quiet logs (no type9 spam), LEFT/RIGHT grids, much lower decoder lag
# type9 keepalive count printed every ~2s
```

## Next
1. Confirm decoder lag drops substantially
2. Pose still absent — needs static RE of SendSensorData trigger
3. Clean up VideoInit spray
4. monado / ALVR / WiVRn later

## Notes
- Base: 6813f93
