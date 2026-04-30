# 3.1.9.4: Notes

This page is subsection **3.1.9.4** of [RFC-183: Chico Vegetation](../../../README.md)


* Use world-space coordinates, not UVs, so bark color stays stable across generated meshes.
* Use species palettes for broad identity and noise for local variation.
* Low-frequency noise should shift between bark tones; high-frequency noise should only modulate value.
* This can be shared by trunks, branches, descenders, and joint-concealing bark balls.

