# TODO / Handoff

## Current tip
- **Connect works.** HELLO_DIRECT + type-0 HelloResponse → succeeded.
- Client sends StartRequest (type 3) and device caps (type 5).
- Black screen: no VideoInit / bitstream yet (`video/` MIME empty).
- Bundle: about to cut 015.1

## Live session parameters (VR-1541F)
- 1440×1440, 60 fps, avc, 50 Mbps, separate eyes, NLS on
- HMD id `/hmd`, controller `/ctrlRight` (Daydream)

## Next code
1. Log and optionally ACK type 3 / 5
2. Emit VideoInit (`video/avc`) after StartRequest
3. Minimal H.264 IDR loop (static image) once channel framing known
4. Document any response opcodes from further RE

## Docs
protocol.md + re-notes.md updated with connect success and live type 3/5 JSON.
