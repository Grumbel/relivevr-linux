# TODO / Handoff

## Status
- StartSensor is **S→C** (cap2 #29); we send it after VideoInit, **before** frames.
- Expect pose (`*** POSE`) after that.

## Test
```bash
cargo run   # or nix run .
# Log order: VideoInit → StartSensor → IDR → …
# Then pose events with orient/pos
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-031.1-startsensor-before-frames-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-031.1-startsensor-before-frames-6813f93.bundle HEAD`

## Notes
- Base: 6813f93
