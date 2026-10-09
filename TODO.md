# TODO / Handoff

## Current tip

- Comprehensive docs written for session continuity.
- Bundle: `relivevr-linux-010.1-docs-handoff-6813f93.bundle`
- Previous code tip: `d11763c` (HelloResponse style variants).

## What works

- [x] Fragment header RE + live confirmation (`field2 == payload length`)
- [x] Control plane = type byte + JSON
- [x] Live HelloRequest from VR-1541F decoded
- [x] Unicast HelloResponse reaches client
- [x] `nix build` / `nix run` / style env vars
- [x] Static RE of StartRequest / VideoInit / VideoData / AudioInit keys
- [x] Channel = small int 0..7

## Blocked / next

1. **Acceptable HelloResponse** — client dies or rediscovers after our reply.
   Bisect with:
   ```bash
   RELIVEVR_STYLE=minimal nix run .
   RELIVEVR_STYLE=echo    nix run .
   RELIVEVR_STYLE=full    nix run .
   RELIVEVR_TYPE=1 RELIVEVR_STYLE=minimal nix run .
   ```
   Success = headset stops broadcasting HelloRequest every ~6s and stays up.

2. **AMD logcat on kill**
   ```bash
   adb logcat -c
   # reproduce
   adb logcat -d | grep -iE 'amd|wirelessgvr|awvr|Hello|Abort|signal|fatal' | tail -100
   ```

3. After Hello accepted: capture StartRequest / session packets (`tcpdump -X`).

4. Channel role map + binary video (NAL) framing.

5. DeviceEvent / pose JSON for controller path.

## Bundle history (keep only tip in artifacts)

- 001–009 superseded
- **`relivevr-linux-010.1-docs-handoff-6813f93.bundle`** ← current

## Notes for next agent

- Read `docs/protocol.md` and `docs/re-notes.md` fully before changing code.
- APK: session `attachments/` or re-download; extract under `/tmp` only.
- Work under a temp tree; copy only finished bundles to artifacts.
- Base commit for bundles remains `6813f93` (short hash in filename).
- Live network used: Linux `192.168.178.48`, headset `192.168.178.33`, port 1235.
