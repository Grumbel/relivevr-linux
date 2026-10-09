# TODO / Handoff

## Current tip
- Commit `e8007ce` – flake provides packages.default / apps.default.
- Bundle: `relivevr-linux-006.1-flake-package-6813f93.bundle`

## Major findings (cumulative)
- Fragment header (15 B, BE) fully known; can parse & construct.
- Control plane = type byte + JSON.
- HelloResponse keys + minimal discovery responder implemented.
- Channel = small int 0–7; ChannelsSupported bool array fills support table.
- StartRequest / VideoInit / VideoData / AudioInit JSON keys recovered.
- Binary video path exists but Channel ID + framing still unknown.

## Open work
1. Confirm HelloResponse against a real headset.
2. Map Channel numbers to roles + binary video framing.
3. Full post-Hello handshake (StartRequest flow).
4. Live capture still highest leverage.

## Next concrete steps
- [x] Fragment header, control JSON, discovery responder
- [x] StartRequest / VideoInit / VideoData key recovery
- [x] flake provides `packages.default` / `apps.default` (nix build / nix run)
- [ ] Real-headset validation of the responder
- [ ] Channel role mapping + binary video framing
- [ ] Pose / DeviceEvent JSON layout

## Bundle history
- 001–005.1 (superseded)
- `relivevr-linux-006.1-flake-package-6813f93.bundle` (this tip)

## Notes
- APK in attachments/. Work under /tmp/relivevr-linux.
- `nix build` → result/bin/relivevr-server
- `nix run` → runs the probe (listens on UDP 1235)
