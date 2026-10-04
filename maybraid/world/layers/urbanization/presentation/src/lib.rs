//! Urbanization presentation helpers. Install goes through [`layer_stack::Present`].

/// Marker for host-presenter subscriptions on [`urbanization_layer_model::Urbanization`].
pub struct UrbanizationHosts;

/// Presents padded replacements for [`urbanization_layer_model::Urbanization`].
pub struct PaddedCells;

#[cfg(test)]
mod tests;
