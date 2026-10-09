# TODO / Handoff

## Status
- Viz runs on main thread; UDP on background tokio.
- Init: force `WINIT_UNIX_BACKEND=x11` if unset, minimal GL config,
  try GL 3.3 → 3.0 → GLES → default; errors are labeled per step.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# or: WINIT_UNIX_BACKEND=x11 RELIVEVR_VIZ=1 cargo run
```

## Next
1. Realtime encode
2. OpenXR stub

## Bundle
`/home/workdir/artifacts/relivevr-linux-044.1-viz-gl-fallbacks-6813f93.bundle`

## Notes
- Base: 6813f93
