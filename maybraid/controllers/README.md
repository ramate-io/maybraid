# Controller boundaries

Input state belongs to a virtual controller entity. `VirtualPadController` combines explicitly assigned devices, keeps the rich `VirtualPad`, and publishes `VirtualPadEvent` with its source and frame sequence. The default host combines keyboard, mouse and the first gamepad; additional gamepads retain separate identities.

`controllers/core` owns source assignment, capture and suspension. It binds each frame once and delivers `UiPadEvent` or `ActorPadEvent` to the selected entity. Derived events preserve their `Routed<T>` envelope. Queue deferred effects through that envelope so an ownership change can reject the old action without retargeting it.

| Owner | State | Input/output |
| --- | --- | --- |
| Virtual controller entity | Devices, rich pad, history, cursor, injected overlay | HID and addressed overlay requests → `VirtualPadEvent` |
| Routing boundary | Per-source assignment, capture, suspension and delivery generation | Pad frames → addressed UI/actor frames and lifecycle |
| Menu controller entity | Focus, repeat, modal/text session | UI frame → menu navigation/action events |
| Actor entity | Control sample, intent frame, stance/gesture state | Actor frame → addressed domain intents and controls |
| Equipment/camera/gesture owner | Domain state and ongoing simulation | Addressed controls/lifecycle → domain effects |
| Application host | Explicit primary source, single shell/map/save selection | Domain events → host navigation and persistence |

`PrimaryVirtualPad` identifies the current single-player host source. It is not a global input snapshot. The existing single-window shell and discovery map intentionally project that source; another source must not silently navigate them or overwrite its save. Scoped controllers can serve additional sources independently without implementing a split-screen or join UI.

A capture or reassignment cancels previous input and requires held controls to be released before the next actor can use them. Suspension instead freezes that actor's ongoing domain state. Input recognition uses the delivered frame's time; ongoing simulation may use the simulation clock while respecting actor-local suspension.
