# ReliveVR Protocol Notes

## Discovery
- Default: UDP broadcast on the local subnet.
- Port: **1235** (both discovery and data).
- Settings override (client-side `app.settings` JSON):
  ```json
  {
    "Connection": {
      "EnableDiscovery": false,
      "Server": "UDP://<PC-IP>:1235"
    }
  }
  ```
  Also supports `TCP://...`, `DatagramSize`, `Network` = "UDP"|"TCP", `Port`.

## Transport / Fragmentation (RE'd)
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
- Multi-fragment messages are reassembled by matching seq / offset / length.
- Higher layer (`StreamFlowCtrlProtocol::PrepareMessage`) takes a `Command::Channel`.

## Channels / Messages
- `awvr::Command::Channel` (typed channels)
- `Communicator::SendMessage(Channel, data, size)`
- Specific helpers: `SendSensorData`, `SendControllerData`, `SendServiceData`
- Receiver: `OnMessageReceived(session, Channel, ..., data, size)`
- Discovery messages after reassembly start with a type byte (0 = discovery request).

## Video path (client)
- Android MediaCodec decoder (`AMediaCodec_*`).
- Supports separate left/right eye processing.
- MIME types: `video/` + `hevc` observed; audio `audio/mp4a-latm`.
- `SubmitSPSPPS`, `SubmitInput`, PTS logging per eye.
- Non-linear / foveated scaling supported on some combos.

## Pose / Controllers
- `SensorEngine::Pose` (quaternion + vectors, timestamps, OEMPoseData)
- `ControllerState`
- Daydream-specific mapping (`DaydreamController`, trackpad emulator).

## Versioning
- `ProtocolVersion`, `ProtocolMinVersion` present in the binary.

## Practical status
- Fragment header is fully known → a Linux probe can parse every packet and construct valid fragments.
- Discovery responder is feasible with a little more work on the Command layer.
- Full video session still requires channel IDs, handshake, and video encapsulation format.
