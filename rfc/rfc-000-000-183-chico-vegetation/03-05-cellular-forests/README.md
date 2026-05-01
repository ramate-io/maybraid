# 3.5: Cellular Forests

This page is subsection **3.5** of [RFC-183: Chico Vegetation](../README.md)

Cellular Forests are the top-level allocation system for Chico vegetation. They divide the world into large forest cells, select a coherent forest layering for each cell, and then instantiate compatible grove layers inside that cell.

A reasonable starting point is:

* **Forest cell size:** `1600m x 1600m`
* **Grove cell size inside a selected forest cell:** typically `200m x 200m`
* **Grove grid per forest cell:** `8 x 8`

Each forest cell chooses a forest layering through Hopscotch, samples forest-level parameter biases, then evaluates layer distributions for ground cover, tufts, understory, lower canopy, and upper canopy. Each selected grove then owns its own grove-cell grid and performs its own Bucket Throw selection and placement.

Subsections:

