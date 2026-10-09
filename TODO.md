# TODO / Handoff

## Current tip
- Commit about to be made: deeper video/session JSON keys + docs.

## Major findings (cumulative)
- Fragment header (15 B, BE) fully known; can parse & construct.
- Control plane = type byte + JSON.
- HelloResponse keys + minimal discovery responder implemented.
- Channel = small int 0–7; ChannelsSupported bool array fills support table.
- StartRequest / VideoInit / VideoData / AudioInit JSON keys recovered
  (resolution, bitrate, codec, per-frame PTS / size metadata, etc.).
- Binary video path exists (VideoReceiverCallback → SubmitSPSPPS / SubmitFrame)
  but Channel ID + on-wire framing still unknown.

## Open work
1. Confirm HelloResponse against a real headset (ProtocolVersion, field2, type byte).
2. Map Channel numbers to roles (which one carries binary video?).
3. Binary video framing (how NALs / SPS-PPS are packetized on the wire).
4. Full post-Hello handshake (StartRequest flow).
5. Live capture remains the highest-leverage next step.

## Next concrete steps
- [x] Fragment header, control JSON, discovery responder
- [x] StartRequest / VideoInit / VideoData key recovery
- [ ] Real-headset validation of the responder
- [ ] Channel role mapping + binary video framing
- [ ] Pose / DeviceEvent JSON layout

## Bundle history
- 001–004.1 (superseded)
- Next: 005.1-video-keys-...

## Notes
- APK in attachments/. Work under /tmp/relivevr-linux.
