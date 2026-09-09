<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="maybraid/assets/iconography/maybraid_logo_readme_dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="maybraid/assets/iconography/maybraid_logo_readme_light.svg">
  <img alt="Maybraid" src="maybraid/assets/iconography/maybraid_logo_readme_dark.svg" width="128">
</picture>

# Maybraid

<p>
  <a href="#players"><img alt="Players" src="https://img.shields.io/badge/Players-9EDBFF?style=for-the-badge&labelColor=141820"></a>
  <a href="#developers"><img alt="Developers" src="https://img.shields.io/badge/Developers-FFDB38?style=for-the-badge&labelColor=141820"></a>
  <a href="#organization"><img alt="Organization" src="https://img.shields.io/badge/Organization-9EDBFF?style=for-the-badge&labelColor=141820"></a>
</p>

<p>
  <a href="https://discord.gg/7Fq4TpJWwt"><img alt="Discord" src="https://img.shields.io/badge/Discord-join-5865F2?style=flat-square&logo=discord&logoColor=white"></a>
  <a href="https://github.com/ramate-io/maybraid/releases"><img alt="Releases" src="https://img.shields.io/github/v/release/ramate-io/maybraid?style=flat-square&label=Release"></a>
  <a href="https://www.youtube.com/channel/UCI_8bMlUe0TllYtrUo36PgA"><img alt="YouTube" src="https://img.shields.io/badge/YouTube-trailers-FF0000?style=flat-square&logo=youtube&logoColor=white"></a>
  <a href="./LICENSE.md"><img alt="License" src="https://img.shields.io/badge/License-source--available-141820?style=flat-square&labelColor=9EDBFF"></a>
</p>

</div>

Maybraid is an exploration and combat game with an arbitrarily-large, procedurally-generated world. The game is in early development. It is currently free to play and source available but not intended for redistribution or unapproved modification. See the [LICENSE](./LICENSE.md) for more details.

> [!WARNING]
> We intend to monetize the game. Hence we are not committed to keeping every part of the source code available.
>
> That being said--if we can find a model that allows us to continue to develop as a source-available or even open-source project--we would love to do so!

You can build the current binary and play directly from this repository:

```shell
nix develop
cargo build -p maybraid --release
./target/release/maybraid
```

> [!NOTE]
> The long term technical fantasia for Maybraid is to integrate it into a decentralized state machine atop the Parabyzantine protocol stack led by the [`gwrdfa` API](https://github.com/ramate-io/gwrdfa). In fact, the initial impetus for this game was to partially to prove the [`gwrdfa` API](https://github.com/ramate-io/gwrdfa) concept.

| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Aevent%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%2Cproposal%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `feature` and `bug` issues. |

## Players

If you are interested in playing Maybraid, check out the following resources:

<p>
  <a href="https://discord.gg/7Fq4TpJWwt"><img alt="Join Discord" src="https://img.shields.io/badge/Discord-community-5865F2?style=for-the-badge&logo=discord&logoColor=white"></a>
  <a href="https://www.youtube.com/channel/UCI_8bMlUe0TllYtrUo36PgA"><img alt="Watch on YouTube" src="https://img.shields.io/badge/YouTube-trailers_%26_tutorials-FF0000?style=for-the-badge&logo=youtube&logoColor=white"></a>
  <a href="https://github.com/ramate-io/maybraid/releases"><img alt="Download releases" src="https://img.shields.io/badge/Releases-download-FFDB38?style=for-the-badge&labelColor=141820"></a>
</p>

To play the game, visit the [Releases](https://github.com/ramate-io/maybraid/releases) page and identify the latest release for your platform.

To build the game from source run:

```shell
nix develop
cargo build -p maybraid --release
./target/release/maybraid
```

## Developers

To use this repository, install [Determinate Systems Nix](https://determinate.systems/blog/determinate-nix-installer/). Then `cd` into the working directory for the repository and `nix develop`.

<p>
  <a href="./CONTRIBUTING.md"><img alt="Contributing guide" src="https://img.shields.io/badge/CONTRIBUTING-guide-9EDBFF?style=for-the-badge&labelColor=141820"></a>
  <a href="https://www.conventionalcommits.org/en/v1.0.0/"><img alt="Conventional Commits" src="https://img.shields.io/badge/commits-conventional-FFDB38?style=for-the-badge&labelColor=141820"></a>
</p>

We have many scoped [CONTRIBUTING.md](./CONTRIBUTING.md) files throughout the repository. Read those as you contribute to a particular system.

We prefer the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) format for commit messages. But, we are not strict about it; commit message content is more important. We have a particular agent-based flow that that the founding team has enshrined in [CONTRIBUTING.md](./CONTRIBUTING.md).

## Organization

<p>
  <a href="./rfc/"><img alt="RFCs" src="https://img.shields.io/badge/rfc-proposals_%26_specs-9EDBFF?style=flat-square&labelColor=141820"></a>
  <a href="./maybraid/"><img alt="Maybraid crates" src="https://img.shields.io/badge/maybraid-game_crates-FFDB38?style=flat-square&labelColor=141820"></a>
</p>

- **[`rfc`](./rfc/):** proposals and specifications providing both institutional memory of the project and the latest designs.
- **[`maybraid`](./maybraid/):** game crates and staged world models.
