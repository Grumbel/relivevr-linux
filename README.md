# relivevr-linux

Experimental reverse-engineering of the AMD ReliveVR (Radeon ReLive for VR / Wireless GVR) protocol, with the goal of a native Linux/NixOS server that can stream frames to a ReliveVR client headset.

## Status
Very early. See `TODO.md` and `docs/`.

## Quick start (once built)
```bash
nix develop
cargo run
# listens on UDP 1235 and logs packets
```

## License
GPL-3.0-or-later
