# 3.4.2.2: Cell Participation

This page is subsection **3.4.2.2** of [RFC-183: Chico Vegetation](../../../README.md)


All grove cells participate in selection. There is no separate activation test that skips a cell before variant selection.

```rust
let selected = bucket_throw(grove.distribution, grove_cell);
```

If a grove needs empty space, `None` should be an explicit item in the grove distribution:

```rust
pub enum ExampleGroveCell {
    None(Bucket {
        weight: 2.0,
        item: None,
    }),
    Tree(Bucket {
        weight: 1.0,
        item: TreeCell,
    }),
}
```

Density still matters, but it should bias the distribution and placement parameters rather than removing cells from the evaluation model. This keeps every grove cell deterministic, makes emptiness authorable, and allows first-fit placement to move through adjacent bucketed variants without fighting a separate activation mask.

