# TODO / Handoff

## Status
- Windows pcap: VideoInit → client StartSensor → pose (no frames first).
- Tip 028: VideoInit only on ctrl caps; arm video on StartSensor.

## Test
```bash
nix run .
# Expect after VideoInit: waiting for StartSensor
# Then hopefully *** StartSensor and pose
# Note: display may stay black until StartSensor arrives
```

## Notes
- Base: 6813f93
