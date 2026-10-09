# TODO / Handoff

## Status
- Video works (dual-eye patterns).
- **Pose blocked**: client never sends `{"Message":"StartSensor"}` to our server.
- Windows pcap shows StartSensor after VideoInit; same order + NUL + SPS/PPS did not unlock it.
- Empty type-4 `{}` = Daydream button only.

## Open gap (StartSensor)
Likely remaining differences vs Windows:
1. Codec path: Windows Hello/VideoInit used **hevc** only; client requests **avc** from us
2. Unknown server→client message not in the short pcap
3. Client binary / Daydream path gating

## Next
1. Capture **our** full session with Wireshark and diff VideoInit vs Windows #7
2. Try Hello `VideoCodecs:["hevc"]` only (may break avc video)
3. Frida: hook where `Message":"StartSensor"` is built/sent
4. Pose JSON parse ready once packets arrive

## Notes
- Base: 6813f93
