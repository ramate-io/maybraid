//! Jungle growth assembly ([RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md), [#226](https://github.com/ramate-io/maybraid/issues/226)).

mod assembly;
mod config;
pub mod render_item_plugin;

pub use assembly::JungleGrowth;
pub use config::JungleGrowthShape;
