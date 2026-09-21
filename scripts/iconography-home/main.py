"""Bake a *_logo_icon .blend to a home-screen PNG (TEXT_YELLOW on MENU_CLEAR).

Invoked headlessly:

    blender --background input.blend --python scripts/iconography-home/main.py -- output.png

Batch wrapper: ``scripts/iconography-home/render.sh``.

Same ortho frame as ``scripts/iconography-png`` (keep camera constants in
sync). After the transparent Workbench pass, opaque pixels become
``TEXT_YELLOW`` and the film composites over ``MENU_CLEAR`` — the home
wash and idle menu yellow from ``maybraid/menu/components/src/theme.rs``.

Does not replace the grayscale HUD mark (``maybraid_logo_icon.png``).

Requires Blender's bundled Python (``bpy``).
"""

import math
import os
import struct
import sys
import zlib

RESOLUTION = 512
ORTHO_SCALE = 2.2
CAMERA_NAME = "IconRenderCamera"
# Blender cameras look along local −Z. At (0, −10, 0) with +90° around X,
# that look axis is world +Y, so the camera views the XZ icon from −Y.
CAMERA_LOCATION = (0.0, -10.0, 0.0)
CAMERA_ROTATION = (math.radians(90.0), 0.0, 0.0)

# maybraid/menu/components/src/theme.rs — Color::srgb, used as-is.
TEXT_YELLOW = (1.0, 0.86, 0.22)
MENU_CLEAR = (0.08, 0.10, 0.14)


def _output_path() -> str:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected output path after '--'", file=sys.stderr)
        sys.exit(1)
    return os.path.abspath(argv[argv.index("--") + 1])


def _ensure_ortho_camera(scene) -> None:
    import bpy

    existing = bpy.data.objects.get(CAMERA_NAME)
    if existing is not None:
        bpy.data.objects.remove(existing, do_unlink=True)

    camera = bpy.data.cameras.new(CAMERA_NAME)
    camera.type = "ORTHO"
    camera.ortho_scale = ORTHO_SCALE
    camera.clip_start = 0.1
    camera.clip_end = 100.0

    camera_obj = bpy.data.objects.new(CAMERA_NAME, camera)
    camera_obj.location = CAMERA_LOCATION
    camera_obj.rotation_euler = CAMERA_ROTATION
    scene.collection.objects.link(camera_obj)
    scene.camera = camera_obj


def _configure_render(scene, output_path: str) -> None:
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = RESOLUTION
    scene.render.resolution_y = RESOLUTION
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.filepath = output_path
    scene.render.use_file_extension = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.image_settings.color_depth = "8"
    scene.render.image_settings.compression = 15

    shading = scene.display.shading
    shading.light = "FLAT"
    shading.color_type = "MATERIAL"
    if hasattr(scene.display, "render_aa"):
        scene.display.render_aa = "32"


def _u8(channel: float) -> int:
    return min(255, max(0, round(channel * 255.0)))


def _paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def _unfilter_rows(raw: bytes, height: int, stride: int, bpp: int, path: str) -> list[bytearray]:
    rows: list[bytearray] = []
    prev = bytearray(stride)
    cursor = 0
    for _ in range(height):
        filt = raw[cursor]
        cursor += 1
        row = bytearray(raw[cursor : cursor + stride])
        cursor += stride
        if filt == 1:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + left) & 255
        elif filt == 2:
            for x in range(stride):
                row[x] = (row[x] + prev[x]) & 255
        elif filt == 3:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + (left + prev[x]) // 2) & 255
        elif filt == 4:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                up_left = prev[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + _paeth(left, prev[x], up_left)) & 255
        elif filt != 0:
            raise ValueError(f"unsupported PNG filter {filt}: {path}")
        rows.append(row)
        prev = row
    return rows


def _read_png_rgba(path: str) -> tuple[int, int, bytearray]:
    with open(path, "rb") as handle:
        if handle.read(8) != b"\x89PNG\r\n\x1a\n":
            raise ValueError(f"not a PNG: {path}")
        width = height = None
        bit_depth = color_type = None
        idat = b""
        while True:
            header = handle.read(8)
            if len(header) < 8:
                raise ValueError(f"truncated PNG: {path}")
            length, tag = struct.unpack(">I4s", header)
            data = handle.read(length)
            handle.read(4)
            if tag == b"IHDR":
                width, height, bit_depth, color_type, *_ = struct.unpack(">IIBBBBB", data)
            elif tag == b"IDAT":
                idat += data
            elif tag == b"IEND":
                break
    if width is None or height is None or bit_depth is None or color_type is None:
        raise ValueError(f"PNG missing IHDR: {path}")
    if bit_depth != 8 or color_type not in (4, 6):
        raise ValueError(
            f"expected 8-bit RGBA or gray+alpha PNG, got depth={bit_depth} type={color_type}: {path}"
        )

    bpp = 4 if color_type == 6 else 2
    stride = width * bpp
    rows = _unfilter_rows(zlib.decompress(idat), height, stride, bpp, path)
    pixels = bytearray()
    if color_type == 6:
        for row in rows:
            pixels.extend(row)
    else:
        for row in rows:
            for x in range(0, stride, 2):
                pixels.extend((row[x], row[x], row[x], row[x + 1]))
    return width, height, pixels


def _write_png_rgba(path: str, width: int, height: int, pixels: bytes) -> None:
    stride = width * 4
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw.extend(pixels[y * stride : (y + 1) * stride])

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as handle:
        handle.write(png)


def _composite_home(pixels: bytearray) -> bytearray:
    out = bytearray(len(pixels))
    yellow = tuple(_u8(c) for c in TEXT_YELLOW)
    wash = tuple(_u8(c) for c in MENU_CLEAR)
    # Workbench film coverage often plateaus below 255. Normalize so the
    # solid mark is exact TEXT_YELLOW and edges keep their relative AA.
    max_alpha = max(pixels[i + 3] for i in range(0, len(pixels), 4)) or 255
    for i in range(0, len(pixels), 4):
        alpha = pixels[i + 3] / max_alpha
        keep = 1.0 - alpha
        out[i] = min(255, max(0, round(wash[0] * keep + yellow[0] * alpha)))
        out[i + 1] = min(255, max(0, round(wash[1] * keep + yellow[1] * alpha)))
        out[i + 2] = min(255, max(0, round(wash[2] * keep + yellow[2] * alpha)))
        out[i + 3] = 255
    return out


def _bake_home_png(path: str) -> None:
    width, height, pixels = _read_png_rgba(path)
    _write_png_rgba(path, width, height, _composite_home(pixels))


def main() -> None:
    output_path = _output_path()
    output_dir = os.path.dirname(output_path)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    import bpy

    scene = bpy.context.scene
    _ensure_ortho_camera(scene)
    _configure_render(scene, output_path)

    result = bpy.ops.render.render(write_still=True)
    if "FINISHED" not in result:
        print(f"main.py: render failed: {result}", file=sys.stderr)
        sys.exit(1)

    if not os.path.isfile(output_path):
        print(f"main.py: output file was not created: {output_path}", file=sys.stderr)
        sys.exit(1)

    try:
        _bake_home_png(output_path)
    except ValueError as error:
        print(f"main.py: {error}", file=sys.stderr)
        sys.exit(1)

    print(f"Rendered {output_path}")


if __name__ == "__main__":
    main()
