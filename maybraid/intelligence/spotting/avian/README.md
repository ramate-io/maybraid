# Spotting Intelligence Avian

`spotting-intelligence-avian` provides the practical Avian backend for
[`spotting-intelligence`](../lib/README.md).

```rust
use bevy::prelude::*;
use spotting_intelligence_avian::SpottingAvianPlugin;

App::new().add_plugins(SpottingAvianPlugin);
```

On each `Update` in `SpottingSystems::Observe`, the plugin:

1. discovers subjects with an Avian sphere query restricted to
   `PhysicsInteractionLayer::Animated`;
2. skips directives already satisfied by fresh contacts;
3. merges discovery work with due contact refreshes;
4. ranks and caps candidates and visibility samples; and
5. tests samples against `PhysicsInteractionLayer::Fixed`, updating successful
   contacts and failure/retry state.

`clear_segment` is public so aiming and firing systems can use the same finite
segment obstruction rule.
