#!/usr/bin/env python3
"""Deterministic, tiny animated density fixture. No artwork or network inputs.

Write one-level uncompressed R16_SFLOAT KTX2 (Khronos basic DFD, linear scalar).
The format descriptor matches the existing static R16Float volume contract.
"""
import hashlib
import json
import math
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "maybraid/assets/vfx/fixtures/volume_puffs"
DFD = bytes.fromhex("2c00000000000000020028000100010000000000020000000000000000000fc000000000000080bf0000803f")


def frame_bytes(frame, size=24):
    raw = bytearray()
    for z in range(size):
        for y in range(size):
            for x in range(size):
                px, py, pz = ((v + .5) / size * 3.2 - 1.6 for v in (x, y, z))
                t = frame / 7
                density = 0.
                # Three separated, asymmetric puffs reveal parallax and temporal motion.
                for cx, cy, cz, radius in [(-.65 + .3*t, -.3 + .6*t, -.5, .5),
                                           (.6, -.35 + .2*t, .6 - .3*t, .42),
                                           (.05, .65 - .25*t, -.1, .36 + .13*t)]:
                    q = ((px-cx)**2 + (py-cy)**2 + (pz-cz)**2) / radius**2
                    density += max(0., 1.-q)**2 * 4.
                assert math.isfinite(density) and density >= 0
                raw.extend(struct.pack('<e', density))
    offset = 152  # 80-byte header + 24-byte level index + 44-byte DFD + 4-byte alignment
    header = b'\xabKTX 20\xbb\r\n\x1a\n' + struct.pack('<13I2Q', 76, 2, size, size, size, 0, 1, 1, 0,
                                                        104, len(DFD), 0, 0, 0, 0)
    return header + struct.pack('<3Q', offset, len(raw), len(raw)) + DFD + bytes(4) + raw


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    frames = [f'frame_{i:02}.ktx2' for i in range(8)]
    hashes = {}
    for i, name in enumerate(frames):
        data = frame_bytes(i)
        (OUTPUT / name).write_bytes(data)
        hashes[name] = hashlib.sha256(data).hexdigest()
    spacing = 3.2 / 24
    transform = [spacing, 0, 0, 0, 0, spacing, 0, 0, 0, 0, spacing, 0,
                 -1.6 + spacing*.5, -1.6 + spacing*.5, -1.6 + spacing*.5, 1]
    manifest = dict(schema_version=1, frames=frames, dimensions=[24]*3,
                    bounds=dict(min=[-1.6]*3, max=[1.6]*3), format='r16_float',
                    density_scale=1., index_to_local=transform, fps=2.,
                    end_behavior=dict(mode='fade', seconds=.5))
    (OUTPUT / 'puffs.volume_flipbook.json').write_text(json.dumps(manifest, indent=2)+'\n')
    (OUTPUT / 'provenance.json').write_text(json.dumps(dict(generator='bin/vfx/generate_volume_fixture.py',
        description='Analytic moving compact-support puffs; generated test data, no source artwork',
        encoding='linear R16_SFLOAT, X fastest, one mip, uncompressed KTX2',
        decoded_gpu_bytes=8*24**3*2, sha256=hashes), indent=2)+'\n')


if __name__ == '__main__':
    main()
