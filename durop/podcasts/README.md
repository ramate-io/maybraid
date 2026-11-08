# Maybraid podcast drops

This directory holds the podcast catalog for [Durop](https://github.com/ramate-io/durop).
The loadable `pack/` contains all 28 podcast slots: 22 requested assets and six
approved alternatives for blocked sources, each with a generated 320-pixel-wide PNG
icon. [SOURCES.md](SOURCES.md) records creators, licenses, exact downloads and
substitutions; [VALIDATION.md](VALIDATION.md) records Durop acceptance checks.
Media and icons are ordinary Git files without LFS.

## Add selected media

Durop v1 plays local files, not provider page URLs. Download the selected media,
prepare a PNG card icon, and record the creator, license/permission and required
attribution in `SOURCES.md`. Import with Python 3 and `ffprobe` on PATH:

```sh
python3 durop/podcasts/import_clip.py seal /path/to/seal.gif /path/to/seal.png
```

The importer checks readable media streams, the selection's media category and
the PNG header, then copies media unchanged and writes a same-basename icon and
Durop clip JSON under `pack/`. It refuses to overwrite existing clip files.
It does not download provider pages or establish permission to redistribute.
Review provenance before committing acquired media. Source media retains its
own licensing; Maybraid's repository license does not grant rights to it.

Templates remain in `pending/` with the `.json.in` suffix. They are outside the
loadable pack because Durop rejects JSON clips with missing assets or icons.
The template media suffix is a suggestion; importing uses the local file's actual
supported extension. Pending templates remain as the original selection record
after import; the corresponding `pack/<category>/<id>.json` indicates readiness.

Reaction visuals use `bottom-right`, `contain`, and one quarter of the display
width. The title swallow is a fullscreen transition: `center`, full width, and
`cover`, with a 6500 ms cap for its six-second sequence. Audio selections omit visual metadata, except `cute-level-up`, whose
requested bottom-right setting is preserved (it remains an audio file).
Clips play once, restart on retrigger, and use Durop's 10-second duration cap.
Adjust individual metadata if a selected clip needs a longer duration, at most
600000 ms. Replacement titles describe the bundled assets; original selection templates keep
their original names and IDs. See the substitution table before choosing a clip.

## Regenerate thumbnails

Run `python3 durop/podcasts/generate_icons.py` with FFmpeg on PATH. Like Durop’s
base reactions catalog, visual thumbnails are unframed source images at 320 pixels
wide with their original aspect ratio. Audio thumbnails are 320 × 180 cyan
waveforms on a transparent background. Thumbnails contain no titles; Durop displays the clip names.
The title-swallow thumbnail uses its title-card frame at 3.9 seconds.
Refresh `SHA256SUMS` after regenerating the icons.

## Load in Durop

For local development, select the absolute path to `durop/podcasts/pack` in
Durop's **Catalog** menu, or run from a separate Durop checkout:

```sh
cargo run -p durop --bin durop --locked -- --pack /absolute/path/to/maybraid/durop/podcasts/pack
```

For a Git-backed catalog, configure the Git repository
`https://github.com/ramate-io/maybraid.git`, the desired branch or commit, and
**Pack folder** `durop/podcasts/pack`. The equivalent launch argument is:

```sh
--pack 'https://github.com/ramate-io/maybraid.git#<branch-or-commit>:durop/podcasts/pack'
```

Pin a prepared revision for each recording session. Durop exports ordinary Git
blobs and rejects unresolved Git LFS pointers; media must be available as real
files in the selected pack. See [Durop's v1 pack documentation](https://github.com/ramate-io/durop/blob/main/docs/packs.md).
