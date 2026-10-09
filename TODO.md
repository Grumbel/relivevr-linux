# TODO / Handoff

## Current tip
- Initial project skeleton (this commit).
- APK extracted and basic string/symbol analysis of `libwirelessvr-lib.so` (arm64).

## Open work
1. Deeper static RE of `libwirelessvr-lib.so`:
   - Command / Channel enums
   - Packet / Fragment headers (FlowCtrlProtocol)
   - Discovery packet format
   - Video frame encapsulation (NAL units? timestamps? eye separation?)
   - Pose / controller serialization
2. Capture real traffic if a Windows + headset setup becomes available (Wireshark on 1235/udp+tcp).
3. Implement minimal discovery responder + video injector (hard-coded H.264 IDR or loop).
4. Nix flake with build for the server binary + RE tools (ghidra? radare2? jadx).
5. Document findings continuously in `docs/`.

## Next concrete steps
- [ ] Extract Java side with jadx (or similar) for settings / JNI surface.
- [ ] Use `nm -C` / `readelf` / strings more thoroughly; look for RTTI / vtables.
- [ ] Sketch C++ / Rust structs for known classes (Communicator, FlowCtrlProtocol::Fragment, etc.).
- [ ] Write a UDP listener that answers discovery and logs everything received.
- [ ] Produce first working "send static image" once the video path is understood.

## Bundle history
- None yet. Next bundle: `relivevr-linux-001.1-...`

## Notes for next agent
- APK is at `/home/workdir/attachments/com.amd.wirelessgvr_...apk` (or copy into repo `third_party/`).
- Extracted copy lives under `/tmp/relivevr/apk_extracted` (ephemeral).
- Prefer working under `/tmp/relivevr-linux` and only copy finished bundles to artifacts.
