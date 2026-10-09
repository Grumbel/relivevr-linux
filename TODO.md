# TODO / Handoff

## Status
- Video + pose + trackpad + OpenGL viz working (main-thread EventLoop).
- Controller gizmo shows trackpad and digital button lamps (menu, vol±, …).

## Test
```bash
nix develop
RELIVEVR_VIZ=1 cargo run
# Press Daydream menu → magenta lamp on controller gizmo; log INPUT …/menu/click
```

## Next
1. Realtime encode: FBO → H.264 → video channel
2. OpenXR / monado stub

## Bundle
`/home/workdir/artifacts/relivevr-linux-045.1-viz-menu-btn-6813f93.bundle`

## Notes
- Base: 6813f93
