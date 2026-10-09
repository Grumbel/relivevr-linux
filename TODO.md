# TODO / Handoff

## Current tip
- Commit about to be made: probe broadcasts Hello every 2s + better diagnostics; flake warning fixed.

## Runtime status
- `nix run` works; listens on 0.0.0.0:1235.
- User reported no incoming packets → almost certainly network path (same subnet / AP client isolation / firewall / headset not discovering).

## Major findings (cumulative)
- Fragment header (15 B, BE) fully known.
- Control plane = type byte + JSON; Hello + StartRequest + Video* keys known.
- Minimal discovery responder + periodic broadcast announce.
- Channel = 0–7; binary video framing still open.

## Open work
1. Get first real packet from a headset (network / app.settings Server=UDP://ip:1235).
2. Refine HelloResponse from observed client request.
3. Map Channel roles + binary video framing.
4. StartRequest / session handshake.

## Notes
- Client initiates discovery (Motor::StartDiscovery / BroadcastMessage).
- Server only needs to reply — or announce if client is passive.
- Manual override on headset: app.settings with Server=UDP://<linux-ip>:1235
