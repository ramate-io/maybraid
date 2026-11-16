# Boss Generation Visual Evidence

Headless frames demonstrating HCSG boss generation and presentation ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).

## Capture

```bash
./maybraid/encounters/bikanwarda/evidence/boss-generation/capture.sh
```

Uses `xvfb-run` with Lavapipe (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vk`) for headless rendering.

## Frames

- `boss-at-origin.png`: World at origin before boss spawn
- `boss-spawned.png`: Generated boss presented via HCSG with #1134 character assembly

**Note:** Placeholder geometry is used until final #1133 GLB assets land. The frames demonstrate:
- HCSG generation contract working
- Boss assembly from bikanwarda-character (#1134)
- Presentation plugin presenting the generated encounter
- Debug `boss spawn` command integration
