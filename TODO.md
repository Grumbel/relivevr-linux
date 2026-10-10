# TODO / Handoff

## Status
- **085.1:** Root cause for `sensor pts not found … prev=0`:
  dumpsmall: **all** VideoData `ptsSensor` values are exact `/hmd/pose` times
  (624/624), never controller. DeviceEvents include both; we were overwriting
  `latest_time` with `/ctrlRight/pose` times → Present HMD sensor lookup miss.
  Fix: only `/hmd/pose` updates `latest_time`; viz stamps `hmd.time`.
- Prior: 084.1 render-time pose stamp, 082.1 ptsSend, no bad connect bursts.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
```
Expect non-zero `ptsSensor` from HMD only; `sensor pts not found` should stop
if this was the sole cause.

## Bundle
`/home/workdir/artifacts/relivevr-linux-085.1-hmd-only-ptssensor-f993e2b.bundle`
