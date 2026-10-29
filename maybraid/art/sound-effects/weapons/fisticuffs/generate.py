#!/usr/bin/env python3
"""Rebuild the original, procedural mono punch impact (no sampled recordings)."""

import math
from pathlib import Path
import random
import struct
import wave


def impact_samples(rate: int = 48_000, duration: float = 0.18) -> list[float]:
    """Combine a short body thump with a filtered, dry contact transient."""
    rng = random.Random(1495)
    low_noise = 0.0
    alpha = 1.0 - math.exp(-2.0 * math.pi * 1800.0 / rate)
    samples = []
    for index in range(round(rate * duration)):
        time = index / rate
        noise = rng.uniform(-1.0, 1.0)
        low_noise += alpha * (noise - low_noise)
        body = (
            0.60 * math.sin(2.0 * math.pi * 110.0 * time)
            + 0.10 * math.sin(2.0 * math.pi * 187.0 * time)
            + 0.06 * math.sin(2.0 * math.pi * 263.0 * time)
        ) * math.exp(-time / 0.032)
        contact = 0.48 * low_noise * math.exp(-time / 0.018)
        snap = 0.12 * (noise - low_noise) * math.exp(-time / 0.003)
        attack = min(1.0, time / 0.0008)
        release = min(1.0, (duration - time) / 0.015)
        samples.append((body + contact + snap) * attack * release)
    # Remove DC, then taper both ends and leave headroom for the impact bus.
    mean = sum(samples) / len(samples)
    samples = [
        (sample - mean) * min(1.0, index / 24, (len(samples) - 1 - index) / 240)
        for index, sample in enumerate(samples)
    ]
    gain = 0.80 / max(abs(sample) for sample in samples)
    return [sample * gain for sample in samples]


def write_clip(path: Path, samples: list[float], rate: int = 48_000) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    pcm = b"".join(struct.pack("<h", round(sample * 32767)) for sample in samples)
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(rate)
        output.writeframes(pcm)


if __name__ == "__main__":
    source = Path(__file__).resolve().parent
    samples = impact_samples()
    write_clip(source / "impact_001.wav", samples)
    write_clip(
        source.parents[3] / "assets/sound-effects/weapons/fisticuffs/impact_001.wav",
        samples,
    )
