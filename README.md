# relivevr-linux

Experimental Linux server for AMD ReliveVR (Wireless GVR) protocol.

## Status (2026-10-09)

| Stage | State |
|-------|--------|
| Discovery + HELLO_DIRECT | Working |
| VideoInit → decoder avc 1440×1440 | Working |
| H.264 frames on channel 1 | Working — **image on headset** |
| Stereo (dual decoder) | Working — both eyes fed (frmType 0+1) |
| Per-eye colour check | **Confirmed** — left blue / right red |
| Pose / controllers | Not started |
| SteamVR / ALVR merge | Future |

## Run

```bash
nix run .
# headset on same LAN; ReliveVR app in discovery mode
```

Env: `RELIVEVR_STYLE`, `RELIVEVR_TYPE`, `RELIVEVR_VIDEOINIT_TYPE`

## Docs

- `docs/protocol.md` — wire format
- `docs/re-notes.md` — RE findings
- `TODO.md` — handoff tip
