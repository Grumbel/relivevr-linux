# TODO / Handoff

## Status
- **Working:** hardware H.264 (h264_vaapi on AMD) live stereo at 1440² × 75 fps,
  ~25 ms end-to-end (client stats), correct lens geometry on Mirage Solo.
  See `docs/re-notes.md` sections from 2026-10-13/14 for the root causes
  (client always decodes avc, skipped P-frames, AU framing, pts pacing,
  FOV mismatch, NonLinearScaling must be false).

## Open
- HEVC: client ignores VideoInit CodecID and keeps its avc decoder. Find what
  makes it request hevc in StartRequest (it reads server `VideoCodecs` in
  `Motor::StartCommunications` 0xddeec; Windows capture had a hevc StartRequest).
- Follow client UpdateRequest `FrameRate` instead of fixed `RELIVEVR_ENCODE_FPS`.
- SteamVR / OpenXR integration: symmetric 100° projection, passthrough
  ComputeDistortion (as the Windows driver does).
