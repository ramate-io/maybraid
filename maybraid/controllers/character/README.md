# Character controller

Maps [`VirtualPad`](../virtual-pad/README.md) onto gameplay intents. Analog is
the current stick / trigger. Digital is a press or release edge. Chords are
resolved here so consumers do not re-read the pad.

Bind observes rich [`ActorPadEvent`](../core/src/delivery.rs) frames after source-scoped
capture resolution. Every delivered frame produces an [`ActorControlEvent`](src/sample.rs)
and actor-owned [`RoutedActorControl`](src/sample.rs), including neutral/release
frames. Physical A / B remain distinct from semantic `Jump`. Samples retain the
source binding, sequence and original frame duration for gesture recognizers.

Semantic [`ActorIntentEvent`](src/intent.rs) values address the same actor and
source. [`CharacterIntentFrame`](src/intent.rs) stores the complete current tick,
including an empty neutral frame, for ordinary locomotion. Derived effects use
`Routed::map(...).queue(...)` or `.trigger(...)` to validate the original binding
when deferred work is actually delivered. Shell actions resolve before physical
and gameplay children, so a same-frame pause cancels pending fire before it can
start a hold. Captured frames emit only explicit shell commands; they cannot
create gameplay physical samples.

[`ActorControlStatus`](src/status.rs) records each actor's source ownership and
connected/captured/suspended state from targeted lifecycle events. Missing status
means an uncontrolled actor. Suspension freezes the owner's ongoing work;
capture, disconnect and reassignment cancel its delivery. Consumers can read
this owned component without querying the global router. Ephemeral input frames
clear before each Bind delivery, so absent frames never replay edges.

The default actor assignment belongs to the host. Semantic controls and physical
state have no process-wide broadcast or singleton mirror. See
[ARCHITECTURE.md](../ARCHITECTURE.md).

`RightTrigger` + `X` is `PowerUseItem` and suppresses `StartInteraction`.
L3 hold is sprint (`StartSprint` every hold frame, `StopSprint` on release).
Shift maps to L3. R3 click swaps first / third person.
LT is optic `Focus`; left bumper is iron `Ads`. **B** tap is `ChangeSquat`,
**B** hold (~0.35 s) is `ChangeProne`. Capture owns **B** as `MenuNav::Back`;
character collect skips stance when that source is captured.
Discover maps read stick flicks in `maybraid-skill-map`, not a bumper chord.
