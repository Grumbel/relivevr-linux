# ReliveVR Protocol Notes

## Discovery
- Default: UDP broadcast on the local subnet, port **1235**.
- Client can override via `app.settings` JSON (`EnableDiscovery`, `Server` = `UDP://ip:1235` or `TCP://...`).

## Transport / Fragmentation
Every datagram that goes through `FlowCtrlProtocol` has a **15-byte header** followed by payload:

```c
struct FragmentHeader {          // multi-byte fields big-endian
    uint16_t seq;                // +0  sequence number
    uint32_t field2;             // +2  (total size / message-related – still tentative)
    uint32_t offset;             // +6  fragment offset in full message
    uint32_t length;             // +10 this fragment's payload length
    uint8_t  flags;              // +14
    // uint8_t payload[length];  // starts at +15
};
```

- Total packet size must equal `length + 15`.
- Multi-fragment messages are reassembled by matching seq/offset/length.

## Control plane (JSON)
After FlowCtrl reassembly:

```
uint8_t type;          // 0 = discovery / Hello family
char    json_data[];   // JSON text
```

### Hello / Discovery
- Type 0 → `HelloRequest` / produce `HelloResponse` or `HelloRefused`.
- Known HelloResponse keys:
  - `ProtocolVersion`, `ProtocolMinVersion`
  - `MaxDatagramSize`, `DeviceID`, `Options`, `ServerName`
  - `ChannelsSupported` (array of ≤8 bools)
  - `Transports` (e.g. `["UDP"]`)

### Channel
`Command::Channel` is a small integer (0–7 observed).  
`ChannelsSupported` JSON array maps directly onto a byte table used by `IsChannelSupported`.

## Video / Audio
- Client: Android MediaCodec (`video/` + `hevc`, `audio/mp4a-latm`).
- Separate L/R eye + non-linear scaling supported.
- Data path is binary (not JSON).

## Current probe capabilities
- Parses every fragment header.
- Prints type + JSON for control messages.
- Replies to type-0 single-fragment probes with a crafted HelloResponse.
