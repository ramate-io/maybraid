Yes, that makes sense. I’d make `progress` the central animation sampling coordinate.

Here’s the spec I’d use.

```rust
/// Applies an animation to a rig at a specific normalized progress value.
///
/// `progress` is the animation's sampling coordinate.
///
/// For cyclic animations, progress wraps by taking `progress.fract()`.
///
/// Examples:
///
/// - `0.0` samples the start of the cycle
/// - `0.5` samples halfway through the cycle
/// - `1.0` samples the start of the next cycle
/// - `1.5` samples halfway through the next cycle
///
/// Non-cyclic animations may clamp or otherwise interpret progress beyond
/// `0.0..=1.0`.
pub trait Animation<R> {
    fn apply(&self, rig: &mut R, progress: f32) -> Effects;
}
```

I’d add a small helper type/convention:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress(pub f32);

impl Progress {
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Cyclic progress in [0.0, 1.0).
    pub fn cycle(self) -> f32 {
        self.0.rem_euclid(1.0)
    }

    /// Clamped one-shot progress in [0.0, 1.0].
    pub fn clamp(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// Whole completed cycles.
    pub fn cycles(self) -> f32 {
        self.0.floor()
    }

    /// True once a one-shot animation has reached or passed its end.
    pub fn is_complete(self) -> bool {
        self.0 >= 1.0
    }
}
```

Then `Run` becomes cleaner:

```rust
impl<R: HumanoidRig> Animation<R> for Run<R> {
    fn apply(&self, rig: &mut R, progress: f32) -> Effects {
        let phase = Progress(progress).cycle();

        let left_arm_swing = -arm_swing(phase);
        let right_arm_swing = arm_swing(phase + 0.5);
        let run = self;

        apply_leg(rig, Side::Left, phase, -1.0, run);
        apply_leg(rig, Side::Right, phase, 1.0, run);

        apply_run_arm(
            rig,
            Side::Left,
            left_arm_swing,
            phase,
            rig.forearm_flex_sign(Side::Left),
            -run.arm_down,
            run,
        );

        apply_run_arm(
            rig,
            Side::Right,
            right_arm_swing,
            phase,
            rig.forearm_flex_sign(Side::Right),
            run.arm_down,
            run,
        );

        Effects::default()
    }
}
```

This likely means `Run` should no longer store `phase`:

```rust
pub struct Run<R> {
    pub stride: f32,
    pub hip_swing: f32,
    pub hip_lift: f32,
    pub knee_neutral: f32,
    pub knee_extended: f32,
    pub knee_contracted: f32,
    pub shoulder_swing: f32,
    pub shoulder_lift: f32,
    pub humerus_swing_scale: f32,
    pub arm_down: f32,
    pub elbow_bend: f32,
    pub elbow_pump: f32,
    pub elbow_cycle: f32,
    _rig: PhantomData<R>,
}
```

Then animation state owns progress:

```rust
pub struct Playing<A> {
    pub animation: A,
    pub progress: f32,
    pub speed: f32,
}
```

And the controller does:

```rust
playing.progress += delta_seconds * playing.speed;
playing.animation.apply(rig, playing.progress);
```

For `Transition`, I’d define it as:

```rust
pub struct Transition<A, R> {
    pub animation: A,
    pub from_pose: RigPose,
    pub curve: TransitionCurve,
    _rig: PhantomData<R>,
}
```

with:

```rust
impl<A, R> Transition<A, R>
where
    A: Animation<R>,
    R: HumanoidRig,
{
    pub fn apply(
        &self,
        rig: &mut R,
        animation_progress: f32,
        transition_progress: f32,
    ) -> Effects {
        let rest = snapshot_pose(rig);

        restore_pose(rig, &rest);
        let effects = self.animation.apply(rig, animation_progress);
        let target_pose = snapshot_pose(rig);

        let weight = self.curve.sample(transition_progress.clamp(0.0, 1.0));

        blend_pose(rig, &self.from_pose, &target_pose, weight);
        mix_effects(Effects::default(), effects, weight)
    }
}
```

Important distinction:

```rust
animation_progress   = where the target animation is in its cycle
transition_progress  = how far the transition blend has completed
```

So usage becomes:

```rust
transition.apply(
    rig,
    run_progress,        // can be 0.0, 0.5, 1.5, 20.25, etc.
    transition_progress, // usually clamped 0.0..=1.0
);
```

I would **not** overload one progress value to mean both things. A run cycle and a transition blend are two different clocks.

Core API rule:

```text
Animation progress:
    cyclic animations wrap
    one-shot animations clamp or complete
    values above 1.0 are valid

Transition progress:
    always 0.0..=1.0
    controls blend weight only
    values above 1.0 mean transition is finished
```

This is a much cleaner model than storing `phase` inside each animation. The animation becomes a pure sampler, while the animation controller owns time.
