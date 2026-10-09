# TODO / Handoff

## Current tip
- Commit `a4ebc7e` – minimal discovery responder + Channel insight.
- Bundle: about to be written as 004.1.

## Major findings
- Fragment header (15 B, BE) fully known.
- Control plane = type byte + JSON.
- HelloResponse keys recovered.
- Channel is a small integer (0–7); ChannelsSupported is a bool array that fills a table at offset 97 in ServerParametersImpl.
- Probe now replies to type-0 discovery probes with a crafted HelloResponse.

## Open work
1. Validate / refine the HelloResponse (ProtocolVersion values, field2 meaning, response type byte) with live traffic or more RE.
2. Map exact Channel numbers to roles (video / audio / sensor / service…).
3. Video framing (NAL encapsulation, SPS/PPS, PTS).
4. Full session handshake after Hello (StartRequest etc.).
5. Live capture still highly valuable.

## Next concrete steps
- [x] Fragment header
- [x] Control = type + JSON
- [x] Probe prints JSON
- [x] Minimal discovery responder implemented
- [ ] Confirm responder works against a real headset (needs user test)
- [ ] Map SERVICE_OP_CODE / Channel roles
- [ ] Video path RE

## Bundle history
- 001.x–003.2 (superseded)
- `relivevr-linux-004.1-discovery-responder-6813f93.bundle` (this tip)

## Notes
- APK in attachments/. Work under /tmp/relivevr-linux.
