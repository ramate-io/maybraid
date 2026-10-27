# Menu controller

Routes [`MenuNavPad`](../../input/README.md) impulses onto one focused menu
inside a scope entity. Put [`MenuController`] on a screen root; descendants
that are `TextMenu` / `HudMenu` are eligible. Overlay menus win over panels.

Delivery is a [`MenuNavImpulse`] on a [`UiTarget`](../core/src/ui.rs) — not a
broadcast to every child. Exclusive modal nav acquires a
[`CaptureToken`](../core/src/capture.rs) on
[`InputRouter`](../core/src/router.rs); widgets apply the impulse;
`MenuActivate` still bubbles up.

This is the phase 1 targeted path. See [ARCHITECTURE.md](../ARCHITECTURE.md).

**B** remains `MenuNav::Back`. Character stance also reads **B** (tap squat /
hold prone) but only when [`CaptureStack`](../core/src/capture.rs) does not
own the local pad. Nested captures release independently; dispatch reads
[`InputRouter::top_capture`](../core/src/router.rs) and does not re-resolve
exclusive nav.

Inventory context actions use `MenuActionImpulse` from `action.rs`. The controller
maps Y to `MenuAction::Unequip` and X to `MenuAction::Drop`, delivering one action
only to the current MenuNav capture owner while retaining the input source. A
simultaneous nav event wins over either action; if X and Y arrive together, Y wins.
Text locks, consumed Back, and higher captures block delivery. The impulse does not
propagate to parent catalogs; the owning inventory UI resolves it against that
menu's selected tile.
