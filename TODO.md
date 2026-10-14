# TODO / Handoff

## Status
- **Working:** hardware H.264 (h264_vaapi on AMD) live stereo at 1440² × 75 fps,
  ~25 ms end-to-end (client stats), correct lens geometry on Mirage Solo.
  See `docs/re-notes.md` sections from 2026-10-13/14 for the root causes
  (client always decodes avc, skipped P-frames, AU framing, pts pacing,
  FOV mismatch, NonLinearScaling must be false).

- **SteamVR:** works through the ALVR fork in `../alvr` (branch `relivevr`):
  headset shows SteamVR home/menu, ~26 ms latency. See docs/re-notes.md
  "SteamVR via ALVR fork".

## Open
- SteamVR "Fail (-203)" status alert and black desktop VR View (rendering works).
- Driver unload watchdog abort on SteamVR shutdown.
- Fork cleanup: debug dump `~/relivevr_left.h264`, CEncoder frame traces;
  exact (not freshest) ptsSensor once pose matching is reliable.
- Controller buttons/trackpad → SteamVR input (poses already mapped).
- Rebase the fork onto ALVR master (direct-mode API, SteamVR 2.17 support).
- HEVC: client ignores VideoInit CodecID and keeps its avc decoder. Find what
  makes it request hevc in StartRequest (it reads server `VideoCodecs` in
  `Motor::StartCommunications` 0xddeec; Windows capture had a hevc StartRequest).
- Follow client UpdateRequest `FrameRate` instead of fixed `RELIVEVR_ENCODE_FPS`.
- SteamVR / OpenXR integration: symmetric 100° projection, passthrough
  ComputeDistortion (as the Windows driver does).
