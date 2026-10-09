# ReliveVR Protocol Notes

## Discovery
- Default: UDP broadcast on the local subnet, port **1235**.
- Client can override via `app.settings` JSON (`EnableDiscovery`, `Server` = `UDP://ip:1235` or `TCP://...`).

## Transport / Fragmentation
Every datagram that goes through `FlowCtrlProtocol` has a **15-byte header** followed by payload:

```c
struct FragmentHeader {          // multi-byte fields big-endian
    uint16_t seq;                // +0  sequence number
    uint32_t field2;             // +2  (total size / message-related)
    uint32_t offset;             // +6  fragment offset in full message
    uint32_t length;             // +10 this fragment's payload length
    uint8_t  flags;              // +14
    // uint8_t payload[length];  // starts at +15
};
```

- Total packet size must equal `length + 15`.
- Multi-fragment messages are reassembled by matching seq/offset/length.
- Higher layer (`StreamFlowCtrlProtocol::PrepareMessage`) takes a `Command::Channel`.

## Control plane (JSON)
After FlowCtrl reassembly, the payload is:

```
uint8_t type;          // 0 = discovery / Hello, 1 = other observed
char    json_data[];   // null-terminated? or length-delimited JSON
```

`Command::ParseBuffer` stores the type byte then feeds the rest to the AMF JSON parser.

### Hello / Discovery messages
Known types:
- `HelloRequest` / `HelloResponse` / `HelloRefused`
- JSON keys observed in `HelloResponse`:
  - `ProtocolVersion`, `ProtocolMinVersion`
  - `MaxDatagramSize`, `DeviceID`, `Options`, `ServerName`
  - `ChannelsSupported`, `Transports`

Other JSON message types present in the binary:
- `StartRequest`, `StopRequest`, `UpdateRequest`
- `VideoForceIDR`
- `DeviceEvent` (carries pose data)
- `TrackableDeviceCaps`, latency stats, etc.

## Video / Audio
- Client uses Android MediaCodec.
- Observed MIME hints: `video/` + `hevc`, `audio/mp4a-latm`.
- Separate left/right eye processing + non-linear scaling supported.
- Video data almost certainly travels on a dedicated binary channel (not JSON).

## Practical status (2026-10-09)
- Fragment header fully known → can parse/construct every UDP packet.
- Control messages are JSON after the type byte → discovery responder is now straightforward.
- Still missing: exact Channel enum values, binary video framing, full session handshake sequence after Hello.
