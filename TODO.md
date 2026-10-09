# TODO / Handoff

## Fix
- Dual-eye: send frames with frmType 0 **and** 1 (both decoder slots).

## Test
```bash
nix run .
# Expect: both eyes solid blue
```

## Next
1. Confirm both eyes blue
2. Optional different colours per eye for verification
3. Pose path
