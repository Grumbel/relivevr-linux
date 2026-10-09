# TODO / Handoff

## Current tip
- Commit `0da2d16` – control plane identified as type-byte + JSON; probe enhanced.
- Bundle: `/home/workdir/artifacts/relivevr-linux-003.1-control-json-6813f93.bundle`
  (base still 6813f93; full cumulative history).

## Major findings this round
- After FlowCtrl reassembly, payload = `uint8 type` + JSON string.
- `Command::ParseBuffer` feeds the JSON part to the AMF JSON parser.
- HelloResponse keys: ProtocolVersion, ProtocolMinVersion, MaxDatagramSize,
  DeviceID, Options, ServerName, ChannelsSupported, Transports.
- Many other control messages are also JSON (Start/Stop/UpdateRequest, VideoForceIDR, DeviceEvent…).

## Open work
1. Finish mapping HelloRequest / HelloResponse field layout and craft a valid discovery reply.
2. Identify Channel enum values (still binary constants).
3. Video channel framing (almost certainly not JSON).
4. Live traffic capture still the fastest path for remaining unknowns.
5. Implement a minimal discovery responder in the probe.

## Next concrete steps
- [x] Fragment header layout
- [x] Control plane identified as type-byte + JSON
- [x] Probe prints JSON when present
- [ ] Craft and send a HelloResponse / discovery reply
- [ ] Map SERVICE_OP_CODE and Channel values
- [ ] Video path RE

## Bundle history
- 001.x / 002.x (superseded)
- `relivevr-linux-003.1-control-json-6813f93.bundle` (current tip)

## Notes
- APK in attachments/. Work under /tmp/relivevr-linux.
