# TODO / Handoff

## Current tip
- Commit `6813f93` – Initial project skeleton.
- Bundle: `/home/workdir/artifacts/relivevr-linux-001.1-initial-skeleton-6813f93.bundle`
  (base = tip = 6813f93; full history is just the root commit).

## Open work
1. Deeper static RE of `libwirelessvr-lib.so`:
   - Command / Channel enums
   - Packet / Fragment headers (FlowCtrlProtocol)
   - Discovery packet format
   - Video frame encapsulation (NAL units? timestamps? eye separation?)
   - Pose / controller serialization
2. Capture real traffic if a Windows + headset setup becomes available (Wireshark on 1235/udp+tcp).
3. Implement minimal discovery responder + video injector (hard-coded H.264 IDR or loop).
4. Expand Nix flake (add Cargo.lock packaging once the binary does more).
5. Document findings continuously in `docs/`.

## Next concrete steps
- [ ] Extract Java side with jadx (download or nix) for settings / JNI surface.
- [ ] Load .so into radare2 / Ghidra; focus on Fragment::ParseFromBuffer and discovery.
- [ ] Sketch C++ / Rust structs for known classes.
- [ ] Improve the UDP listener to attempt a discovery reply once the format is known.
- [ ] Produce first working "send static image" once the video path is understood.

## Bundle history
- `relivevr-linux-001.1-initial-skeleton-6813f93.bundle` (this tip)

## Notes for next agent
- APK lives at `/home/workdir/attachments/com.amd.wirelessgvr_1.0.13-13_minAPI24(arm64-v8a,armeabi-v7a)(nodpi)_apkmirror.com.apk`.
- Extracted analysis was done under `/tmp/relivevr/apk_extracted` (ephemeral; re-extract if needed).
- Work under `/tmp/relivevr-linux` (or re-clone from the bundle), copy only finished bundles to artifacts.
- Next bundle number: 002.x (or 001.2 if only small polish).
