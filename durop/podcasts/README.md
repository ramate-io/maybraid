# Maybraid podcast drops

This directory holds the podcast catalog for [Durop](https://github.com/ramate-io/durop).
The initial 26 unique selections and their source links are in [SOURCES.md](SOURCES.md).
**No media has been downloaded yet. The loadable `pack/` currently has zero clips.**
Acquisition is blocked by this environment's outbound network policy.

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

Visual selections use `bottom-right`, `contain`, and one quarter of the display
width. Audio selections omit visual metadata, except `cute-level-up`, whose
requested bottom-right setting is preserved (it remains an audio file).
Clips play once, restart on retrigger, and use Durop's 10-second duration cap.
Adjust individual metadata if a selected clip needs a longer duration, at most
600000 ms. YouTube titles and the Openverse title are provisional until inspected.

## Load in Durop

For local development, select the absolute path to `durop/podcasts/pack` in
Durop's **Catalog** menu, or run from a separate Durop checkout:

```sh
cargo run -p durop --bin durop --locked -- --pack /absolute/path/to/maybraid/durop/podcasts/pack
```

After committing the pack, configure the Git repository
`https://github.com/ramate-io/maybraid.git`, the desired branch or commit, and
**Pack folder** `durop/podcasts/pack`. The equivalent launch argument is:

```sh
--pack 'https://github.com/ramate-io/maybraid.git#<branch-or-commit>:durop/podcasts/pack'
```

Pin a prepared revision for each recording session. Durop exports ordinary Git
blobs and rejects unresolved Git LFS pointers; media must be available as real
files in the selected pack. See [Durop's v1 pack documentation](https://github.com/ramate-io/durop/blob/main/docs/packs.md).
