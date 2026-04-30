# 3.2: L-system Trees

This page is subsection **3.2** of [RFC-183: Chico Vegetation](../README.md)


L-systems are a well-established method for generating botanical structures and offer a natural way to express recursive growth, branching grammars, and species variation. They are a strong candidate for future expansion of the vegetation system.

However, we avoid adopting L-systems at this stage for a few practical reasons.

Primarily, L-systems do not provide direct spatial implications. While they are excellent for describing *connectivity* and *growth rules*, they do not inherently encode:

* world-space positioning
* spatial ownership or containment
* chunk alignment or LOD boundaries

As a result, additional interpretation layers are required to map symbolic structures into spatial ones. This introduces a tradeoff:

* **Composability**: combining multiple growth rules and structures cleanly
* **Connectivity**: maintaining coherent, continuous geometry in world space

Naive approaches tend to struggle to satisfy both simultaneously. Either the system becomes difficult to compose across species and features, or spatial coherence becomes fragile and expensive to maintain.

In contrast, the ball-stick and radial projection constructions used here:

* operate directly in world space
* align naturally with chunking and LOD systems
* provide predictable spatial ownership
* remain easy to compose and parameterize

While less expressive than full L-systems, they are significantly more practical for initial terrain-scale vegetation.

L-systems remain an important area of future work. In particular, hybrid approaches that:

* retain spatial grounding
* incorporate limited grammar-based growth
* or use L-systems as local refinements within existing structures

...may offer a path to richer vegetation without sacrificing system ergonomics or performance.

