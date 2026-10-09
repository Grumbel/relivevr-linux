# ReliveVR Protocol Notes

## Discovery
- Default: UDP broadcast on the local subnet.
- Port: **1235** (both discovery and data?).
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

## Transport
- UDP (preferred) or TCP.
- Fragmentation / reliability layer: `awvr::FlowCtrlProtocol`
  - Classes: `Fragment`, `Buffer`
  - Methods: `FragmentMessage`, `ProcessFragment`, `SendNextMessage`, `PurgeStaleBuffers`
  - Suggests sequence numbers, fragment IDs, ACKs or windowing.

## Channels / Messages
- `awvr::Command::Channel` (typed channels)
- `Communicator::SendMessage(Channel, data, size)`
- Specific helpers:
  - `SendSensorData`
  - `SendControllerData`
  - `SendServiceData`
- Receiver: `OnMessageReceived(session, Channel, ..., data, size)`
- `ChannelsSupported` / `IsChannelSupported`

## Video path (client)
- Android MediaCodec decoder (`AMediaCodec_*`).
- Supports separate left/right eye processing.
- MIME types for video; likely `video/avc` or `video/hevc`.
- `SubmitSPSPPS`, `SubmitInput`, PTS logging per eye.
- Non-linear / foveated scaling supported on some combos.

## Pose / Controllers
- `SensorEngine::Pose` (quaternion + vectors, timestamps, OEMPoseData)
- `ControllerState`
- Daydream-specific mapping (`DaydreamController`, trackpad emulator).

## Versioning
- `ProtocolVersion`, `ProtocolMinVersion` present in the binary.

## Open questions (high priority)
- Exact discovery packet magic / layout.
- FlowCtrlProtocol fragment header size and fields.
- How video NALs are packetized (one NAL per message? length-prefixed? timestamps?).
- Channel ID values and Command opcodes.
- Whether audio is present in 1.0.13 and how it is framed.
