# RFC-n: Marazion Watersheds

## 1: Motivation

## 2: Prior Art

## 3: Design

The watershed designs proposed in this RFC are referred to as Marazion watersheds. All following the stamping framework proposed in [RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain). 

### 3.1: Marazion Pocket Water Stamping

Marazion Pocket Waters are used to satisfy the [Jersey Pocket Waters requirement of RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#384-jersey-pocket-waters-small-hydrology-chains). 

Marazion pocket waters rely on three levels of cellular stamping hierarchy:

1. **Pre-pocket Cells:** the base parent cells representing the extents within which **Pocket Cells** are generated. For simplicity, they are AABB cells and fix one cell size for all **Pocket Cells** contained within them, creating a grid. The role of **Pre-pocket Cell:** is to vary the extents of **Pocket Cells**. 
2. **Pocket Cells:** the cells within which certain simple hydrology types are selected. A **Pocket Cell** contains a grid within which it fills with independent pocket water types. 
3. **Pocket Water Cells:** the cells within which independent pocket water types are generated. 

### 3.2: Marazion Basin Water Stamping

### 3.3: Marazion Hydrology Complex Stamping

### 3.4: Marazion Global Ocean

## 4: Milestones