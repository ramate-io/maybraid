# Menu controller

Routes [`MenuNavPad`](../../input/README.md) impulses onto one focused menu
inside a scope entity. Put [`MenuController`] on a screen root; descendants
that are `TextMenu` / `HudMenu` are eligible. Overlay menus win over panels.

Delivery is a [`MenuNavImpulse`] on the focused entity — not a broadcast to
every child. Widgets apply the impulse; `MenuActivate` still bubbles up.
