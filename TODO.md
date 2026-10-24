# TODO / Handoff

## Status
- **Working:** hardware H.264 (h264_vaapi on AMD) live stereo at 1440² × 75 fps,
  ~25 ms end-to-end (client stats), correct lens geometry on Mirage Solo.
  See `docs/re-notes.md` sections from 2026-10-13/14 for the root causes
  (client always decodes avc, skipped P-frames, AU framing, pts pacing,
  FOV mismatch, NonLinearScaling must be false).

- **SteamVR:** works through the ALVR fork
  [alvr-relivevr](https://github.com/Grumbel/alvr-relivevr) (branch `master`):
  headset shows the SteamVR environment and menu, ~26 ms latency, Daydream
  controller as Vive wand. Fork-specific open items live in that repo's
  TODO.md. See docs/re-notes.md "SteamVR via ALVR fork".

## Open
- Jitter at low game fps (seen in alvr-relivevr). Reproduce with the low-fps
  simulation: `RELIVEVR_SIM_FPS=30 nix run .#viz`, compare
  `RELIVEVR_SIM_MODE=drop` (ALVR behaviour) against `repeat`, and
  `RELIVEVR_PTS_SENSOR=latest` against the render-pose default. A static scene
  should stay still in every mode if the client reprojects correctly.
- HEVC: client ignores VideoInit CodecID and keeps its avc decoder. Find what
  makes it request hevc in StartRequest (it reads server `VideoCodecs` in
  `Motor::StartCommunications` 0xddeec; Windows capture had a hevc StartRequest).
- Follow client UpdateRequest `FrameRate` instead of fixed `RELIVEVR_ENCODE_FPS`.
- OpenXR (Monado/WiVRn) instead of SteamVR: maybe later. Needs the same
  symmetric 100° projection with no PC-side distortion as the ALVR fork.
