# Typed Character UI Menu Spec

## Implementation Status

The typed character UI now follows the crate split described below:

- `character-ui-menu` owns renderer-independent primitives (`Section`, `Slider`, `SingleSelect`, `SwatchSingleSelect`, `AssetSingleSelect`, `MultiSelect`), `CameraFocus`, and generic `MenuEvent`s.
- `crozon-character-ui-menus` owns Crozon-specific menu traits, trait impls for character enums, typed `BraidmanMenu` / `BrodlerMenu` / `CharacterMenu` trees, menu-to-config conversion, and focus metadata.
- `bevy-character-ui-menu-renderer` owns the Bevy widget helpers for sections, sliders, swatches, asset selects, multi-selects, and menu event buttons.
- `character-concepts-playground/src/ui.rs` is now integration glue: it builds typed menus from `ConceptPreviewConfig`, delegates rendering, applies menu events back to configs, and keeps the existing camera/session/scroll systems alive.

Shared math types use Bevy crates (`bevy_math::Vec3`) rather than an extra standalone `glam` dependency.

## Goal

Replace redundant hand-written character creation UI builders with a typed menu description layer.

The menu system should mirror the backend character definition structure while remaining independent from any particular renderer. The renderer should consume typed menu primitives such as `Section`, `SingleSelect`, `SwatchSingleSelect`, `Slider`, and `AssetSelect`.

The design should avoid a fully dynamic `serde_json::Value` tree as the primary model. Dynamic values may still be produced at the backend boundary, but the menu itself should be strongly typed.

---

## Architecture

The pipeline is:

```text
Character definitions
    → build typed character menu
        → renderer consumes menu primitives
            → Feed back through menu to character definition
```

The intended crate layout is:

```text
crozon-characters/
    lib.rs
    characters/
        character_name.rs
        character_name/
            various.rs
            submodules.rs
            for_constructing_and_rendering_from.rs
            definition.rs // the all important character definition

character-ui-menu/
    lib.rs
    primitives.rs
    primitives/
        multi_select.rs
        single_select.rs
        slider.rs
        section.rs
        swatch.rs
        asset.rs
    camera_focus.rs
    menu.rs
    

crozon-character-ui-menus/
    lib.rs
    characters.rs
    characters/
        character_name.rs // the character-ui-menu meets specific character definitions

bevy-character-ui-menu-renderer/
    lib.rs
    asset_loader.rs
    renderer/
        multi_select.rs
        single_select.rs
        slider.rs
        section.rs
        swatch.rs
        asset.rs

playground/
    main.rs
    lib.rs
    create_a_character/
        ui.rs // bring the pipeline to life renderer meets menu meets character definition
```

---

## Crate Responsibilities

### `crozon-characters`

Owns backend character definitions.

This crate defines the actual character data and construction logic. It should not know about frontend widgets.

Example:

```rust
pub struct Dog {
    pub body: DogBody,
    pub teeth: DogTeeth,
    pub color: DogColor,
}
```

This is the canonical backend model.

---

### `character-ui-menu`

Owns generic typed menu primitives.

This crate defines reusable UI-description types only. It should not know about Bevy rendering.

Examples:

```rust
pub struct Section<T> {
    pub label: &'static str,
    pub value: T,
    pub camera_focus: Option<CameraFocus>,
}
```

```rust
pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub camera_focus: Option<CameraFocus>,
}
```

```rust
pub struct VecSelect<T> {
    pub options: Vec<T>,
    pub selected_index: usize,
    pub camera_focus: Option<CameraFocus>,
}
```

```rust
pub struct SingleSelect<T> {
    pub value: T,
    pub camera_focus: Option<CameraFocus>,
}

/// Can implement over anything that can list its variants and take a string to identify one. 
/// Enums readily can implement these. Allowing lower orders to stay type-safe.
impl <T: FromStr + ToString + ListValues> SingleSelect<T> {
    // ...
}
```

The key point: menu items carry their current/default value directly. Avoid required methods like `default_value()`. Defaults should be on the menu items themselves. Menu item are data. 

---

### `crozon-character-ui-menus`

Owns the frontend menu definitions for Crozon characters.

This crate mirrors the backend character types using `character-ui` primitives.

Example:

```rust
pub struct DogBodyMenu {
    pub body: AssetSingleSelect<DogBodyAsset>, // At this point, the frontend will need to obtain asset ids, so could be Vec<AssetId> or you could allow DogBodyAsset to be an enum that implements  something like ListValues and FromAssetId, same kind of pattern as the typical single select, just different dipslay. 
    pub width: Slider,
}
```

```rust
pub struct DogTeethMenu {
    pub length: Slider,
}
```

```rust
pub enum DogColorMenu {
    Blue,
    Brown,
    Black,
}
```

```rust
pub struct DogMenu {
    pub body: Section<DogBodyMenu>,
    pub teeth: Section<DogTeethMenu>,
    pub color: SwatchSingleSelect<DogColorMenu>,
}
```

```rust
pub enum SpeciesMenu {
    Dog(Section<DogMenu>),
    Cat(Section<CatMenu>),
}
```

```rust
pub struct CharacterMenu {
    pub root: SingleSelect<SpeciesMenu>,
}
```

This gives us a typed taxonomy:

```text
CharacterMenu
└── Species
    ├── Dog
    │   ├── Body
    │   ├── Teeth
    │   └── Color
    └── Cat
        ├── Body
        ├── Tail
        └── Color
```

Each branch can expose a different menu subtree while still composing under a single select.

---

### `bevy-character-ui-menu-renderer`

Owns the rendering implementation.

This crate defines a Bevy plugin that knows how to render `character-ui` primitives.

It should implement rendering for:

```rust
Section<T>
Slider
SingleSelect<T>
VecSelect<T>
SwatchSingleSelect<T>
AssetSingleSelect<T>
```

It may use traits such as:

```rust
pub trait LabelOption {
    fn label(&self) -> &'static str;
}
```

```rust
pub trait SwatchOption: LabelOption {
    fn color(&self) -> Color;
}
```

```rust
pub trait AssetOption: LabelOption {
    fn asset(&self) -> IdentifiedAsset;
}
```

The renderer should select its UI behavior based on the primitive type and supported traits, not by inspecting arbitrary JSON.

---

### `playground`

Owns integration and experimentation.

This crate wires together:

```text
crozon-characters
crozon-character-ui-menus
bevy-character-ui-menu-renderer
```

The create-a-character screen should live here initially:

```text
playground/create_a_character/ui.rs
```

---

## Select Types

The system should support more than one kind of select.

A `VecSelect<T>` is useful when options are runtime-provided:

```rust
pub struct VecSelect<T> {
    pub options: Vec<T>,
    pub selected_index: usize,
}
```

A `SingleSelect<T>` over an enum-like type is useful when options are known statically:

```rust
pub struct SingleSelect<T> {
    pub value: T,
}
```

For enum-backed selects, require traits like:

```rust
pub trait ListValues: Sized {
    fn values() -> &'static [Self];
}
```

```rust
pub trait StringIdentified {
    fn id(&self) -> &'static str;
}
```

Types may also implement `FromStr` and `ToString` / `Display` when string round-tripping is useful.

Example:

```rust
pub enum DogColorMenu {
    Blue,
    Brown,
    Black,
}

impl ListValues for DogColorMenu {
    fn values() -> &'static [Self] {
        &[Self::Blue, Self::Brown, Self::Black]
    }
}

impl StringIdentified for DogColorMenu {
    fn id(&self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Brown => "brown",
            Self::Black => "black",
        }
    }
}
```

Then the renderer can determine:

```text
available options = T::values()
current option = select.value
selected id = select.value.id()
```

---

## Swatches

A swatch select should be a typed select over a type that satisfies a swatch contract.

```rust
pub struct SwatchSingleSelect<T> {
    pub value: T,
    pub camera_focus: Option<CameraFocus>,
}
```

```rust
pub trait SwatchOption {
    fn label(&self) -> &'static str;
    fn color_hex(&self) -> &'static str;
}
```

Example:

```rust
pub enum DogColorMenu {
    Blue,
    Brown,
    Black,
}

impl SwatchOption for DogColorMenu {
    fn label(&self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Brown => "Brown",
            Self::Black => "Black",
        }
    }

    fn color_hex(&self) -> &'static str {
        match self {
            Self::Blue => "#0096FF",
            Self::Brown => "#7A3E1D",
            Self::Black => "#111111",
        }
    }
}
```

---

## Asset Selects

Asset-backed selects should bridge the UI to renderable previews.

```rust
pub struct IdentifiedAsset {
    pub id: &'static str,
    pub label: &'static str,
    pub path: &'static str,
}
```

```rust
pub struct AssetSingleSelect<T> {
    pub value: T,
    pub camera_focus: Option<CameraFocus>,
}
```

```rust
pub trait AssetOption {
    fn label(&self) -> &'static str;
    fn asset(&self) -> IdentifiedAsset;
}
```

This supports fields like:

```rust
pub struct DogBodyMenu {
    pub body: AssetSingleSelect<DogBodyAsset>,
    pub width: Slider,
}
```

---

## Camera Focus

Camera focus belongs in `character-ui`, not the backend character definition.

It is UI-level metadata: geometry-relative, but not part of the actual character data model.

```rust
pub struct CameraFocus {
    pub rig: SocketRig,
    pub socket: &'static str,
    pub camera_offset: Vec3,
    pub look_at_offset: Vec3,
}
```

Menu primitives may optionally carry camera focus:

```rust
pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub camera_focus: Option<CameraFocus>,
}
```

```rust
pub struct Section<T> {
    pub label: &'static str,
    pub value: T,
    pub camera_focus: Option<CameraFocus>,
}
```

When the renderer clicks or focuses a field, it may ask the menu item for its `CameraFocus`.

---

## Defaults

Defaults should be composed directly into the menu values.

Avoid:

```rust
fn default_value() -> T;
```

Prefer:

```rust
pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}
```

and:

```rust
DogBodyMenu {
    body: AssetSingleSelect {
        value: DogBodyAsset::Default,
        camera_focus: Some(...),
    },
    width: Slider {
        value: 1.0,
        min: 0.7,
        max: 1.3,
        step: 0.01,
        camera_focus: Some(...),
    },
}
```

The menu itself is the default state.

---

## Backend Conversion

The menu should eventually convert into the backend character definition.

For example:

```rust
impl From<DogMenu> for Dog {
    fn from(menu: DogMenu) -> Self {
        Self {
            body: DogBody {
                body: menu.body.value.body.value.into(),
                width: menu.body.value.width.value,
            },
            teeth: DogTeeth {
                length: menu.teeth.value.length.value,
            },
            color: menu.color.value.into(),
        }
    }
}
```

JSON may still exist at the boundary, but it should not be the primary internal representation.

The preferred flow is:

```text
Typed menu
    → typed character config
        → optional serialized JSON/network payload
```

---

## Design Preference

Prefer this:

```rust
pub struct DogMenu {
    pub body: Section<DogBodyMenu>,
    pub teeth: Section<DogTeethMenu>,
    pub color: SwatchSingleSelect<DogColorMenu>,
}
```

over this:

```rust
pub struct MenuNode {
    pub kind: MenuNodeKind,
    pub value: serde_json::Value,
    pub children: Vec<MenuNode>,
}
```

The dynamic tree is more flexible, but it throws away too much useful type information. Crozon’s menu is strongly related to known backend character definitions, so the typed approach should be the primary design.

A dynamic representation can still be introduced later as a lowered/intermediate form if needed for reflection, scripting, modding, or editor tooling.

---

## Implementation Principle

The renderer should not own character meaning.

The character menu should not own rendering behavior.

The backend character definition should not own UI widgets.

The clean separation is:

```text
crozon-characters
    meaning and construction

character-ui
    generic typed menu vocabulary

crozon-characters-ui
    Crozon-specific typed menus

bevy-character-ui-renderer
    visual rendering of menu vocabulary

playground
    integration and iteration
```
