# Maybraid

- [Players](#players)
- [Developers](#developers)
- [Organization](#organization)

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
> The long term technical fantasia for Maybraid is to integrate it into a decentralized state machine atop the Parabyzantine protocol stack led by the [`gwrdfa` API ](https://github.com/ramate-io/gwrdfa). In fact, the initial impetus for this game was to partially to prove the [`gwrdfa` API ](https://github.com/ramate-io/gwrdfa) concept. 


| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Aevent%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%2Cproposal%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `feature` and `bug` issues. |

## Players

If you are interested in playing Maybraid, check out the following resources:

- [Discord](https://discord.gg/7Fq4TpJWwt)
- [Trailers, Gameplay Walkthroughs, and Tutorials](https://www.youtube.com/channel/UCI_8bMlUe0TllYtrUo36PgA)

To play the game, visit the [Releases](https://github.com/ramate-io/maybraid/releases) page and identify the latest release for your platform. 

To build the game from source run: 

```shell
nix develop
cargo build -p maybraid --release
./target/release/maybraid
```

## Developers

To use this repository, install [Determinate Systems Nix](https://determinate.systems/blog/determinate-nix-installer/). Then `cd` into the working directory for the repository and `nix develop`.

We have many scoped [CONTRIBUTING.md](./CONTRIBUTING.md) files throughout the repository. Read those as you contribute to a particular system. 

We prefer the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) format for commit messages. But, we are not strict about it; commit message content is more important. We have a particular agent-based flow that that the founding team has enshrined in [CONTRIBUTING.md](./CONTRIBUTING.md).

## Organization

- **[`rfc`](./rfc/):** proposals and specifications providing both institutional memory of the project and the latest designs.
- **[`maybraid`](./maybraid/):** game crates and staged world models. 
