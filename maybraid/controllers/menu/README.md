# Menu controller

The central routing boundary delivers a rich `UiPadEvent` to one focused UI entity.
The menu interpreter reads only that authorized frame. It keeps independent repeat
clocks, sequence checks, and binding provenance on the owning `MenuController`.
It does not poll hardware, the global pad, the router, or primary modal resources.

Attach `MenuController::for_source(source)` to a UI root. The plugin adds a
`MenuSession` when absent; hosts may supply one with their own modal stack and
text state. HUD/text menus beneath that root are eligible for focus. Overlay menus
win over panels; a modal owns focus only inside its own session. An awaiting modal
keeps its source captured while its UI is being built. Removing the UI retires its
capture centrally.

`MenuNav`, `MenuNavImpulse`, inventory `MenuActionImpulse`, and addressed
`MenuBackRequested` belong to `menu-components`. Routed impulses retain their
original authorization. Widget adapters use `impulse.deliver(commands, event)`
for deferred domain actions. A shell host uses `back.queue(commands, action)` or
`back.deliver(commands, event)` so a retired capture cannot replay queued Back.
Standalone keyboard/pointer adapters use the local `::new` constructors.

Back wins over navigation and context actions. Navigation wins over inventory
Drop/Unequip; Unequip wins if X and Y arrive together. Held context buttons are
one-shot. A replaced binding must release held controls before the new session
accepts them. Set `controller.start = MenuStartPolicy::Shell` on a pause root to
leave Start available to the shell; submenu Start selects normally.

Scoped text sessions own their own values, keypad UI, submit, and dismissal.
Scoped inventory viewports use `InventoryMenuSession` in the renderer, preserving
stable bag identity through reorder and cleaning up only the addressed menu.

`primary` is the explicit compatibility adapter for `MenuController::default()`.
It bridges the existing primary shell's `MenuNavPad`, text lock, modal stack, and
keyboard/IME host. Those singleton resources do not gate scoped input. The virtual
pad crate owns rich source state and has no menu navigation or repeat surface.

This implements [issue 1503](https://github.com/ramate-io/maybraid/issues/1503).
