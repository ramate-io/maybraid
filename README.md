# Maybraid

- [General use](#general-use)
- [Playgrounds](#playgrounds)
- [Organization](#organization)

A game of procedural generation and peer-based state.

> [!NOTE]
> Maybraid is currently in very early development. Layer playgrounds sit next to their crates; assembled world runs as `maybraid-world-playground`.

| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Aevent%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%2Cproposal%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `feature` and `bug` issues. |

## General use

To use this repository, install [Determinate Systems Nix](https://determinate.systems/blog/determinate-nix-installer/). Then `cd` into the working directory for the repository and `nix develop`.

## Playgrounds

Assembled world:

```shell
cargo run -p maybraid-world-playground
```

Single-layer playgrounds live next to their crates. Retired apps and how to restore them: [`maybraid/PLAYGROUNDS.md`](./maybraid/PLAYGROUNDS.md).

## Organization

- **[`rfc`](./rfc/):** proposals and specifications providing both institutional memory of the project and the latest designs.
- **[`maybraid`](./maybraid/):** game crates (Durham, Chico, Richmond, Crozon, world). Shared procedural primitives live under [`maybraid/procedural`](./maybraid/procedural/) (`common`, `comproc`).
