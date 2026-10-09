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

## Bundle
`/home/workdir/artifacts/relivevr-linux-028.1-frames-after-startsensor-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-028.1-frames-after-startsensor-6813f93.bundle HEAD`
Tip: 574a59c
