# TODO / Handoff

## MILESTONE: IMAGE ON HEADSET
- Left eye blue, right dark (mono full-frame was only lighting left).
- New tip: SBS 1440×1440 (left blue, right red), PTS timestamps, ~30 fps stream.

## Test
```bash
nix run .
# Expect left blue, right red if SBS layout matches client
```

## Next
1. Confirm stereo layout (SBS vs separate eyes vs mono per eye)
2. Tune bitrate / fps to stop "Decoder input is full"
3. Pose / controllers / SteamVR path
