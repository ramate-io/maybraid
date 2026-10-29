# Punch impact

`impact_001.wav` is an original procedural dry thump with a short contact snap.
It contains no sampled recordings. The source and runtime exports are identical
48 kHz, mono, 16-bit PCM WAVs, lasting 180 ms with peak amplitude capped at 0.8.

Regenerate both exports from the repository root with:

```sh
python3 maybraid/art/sound-effects/weapons/fisticuffs/generate.py
```

The gameplay presentation plays this clip on confirmed fist contact, including
solid world geometry. Effort grunts remain separate from the landing sound.
