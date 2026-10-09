# ReliveVR Protocol Notes

## Discovery (live-confirmed)

Client broadcasts HelloRequest; server unicasts HelloResponse.

**Fragment header** (15 bytes, multi-byte fields big-endian):

```c
struct FragmentHeader {
    uint16_t seq;     // +0  client discovery uses 0
    uint32_t field2;  // +2  == payload length for single-fragment msgs
    uint32_t offset;  // +6  0 for single fragment
    uint32_t length;  // +10 payload length
    uint8_t  flags;   // +14 0 observed
    // payload follows at +15
};
```

**HelloRequest** (type byte 0 + JSON), example from VR-1541F:

```json
{
  "DeviceID": "4b94589deb5e1561",
  "MaxDatagramSize": 65507,
  "Options": {"DeviceType": {"Type": "string", "Val": "VR-1541F"}},
  "ProtocolMinVersion": 1,
  "ProtocolVersion": 1
}
```

**HelloResponse** (type byte 0 + JSON) — keys from client binary + live test:

```json
{
  "ProtocolVersion": 1,
  "ProtocolMinVersion": 1,
  "MaxDatagramSize": 65507,
  "DeviceID": "…",
  "Options": {"DeviceType": {"Type": "string", "Val": "PC"}},
  "ServerName": "…",
  "ChannelsSupported": [true, true, true, true, true, true, true, true],
  "Transports": ["UDP"]
}
```

## Control plane
After fragment reassembly: `uint8 type` + JSON (AMF JSONParser).
Other messages: StartRequest, StopRequest, VideoInit, VideoData, AudioInit,
VideoForceIDR, DeviceEvent, …

## Binary video
VideoReceiverCallback → SubmitSPSPPS / SubmitFrame. Channel ID + NAL framing TBD.

## Channel
Small integer 0–7; ChannelsSupported bool[8] fills IsChannelSupported table.
