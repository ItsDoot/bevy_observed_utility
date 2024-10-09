use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};

use crate::{ecs::CommandsExt, event::OnScore, scoring::Score};

/// [`Score`] [`Component`] that uses an [`Evaluator`] to score a single child entity.
///
/// # Provided Evaluators
///
/// - [`LinearEvaluator`]: A linear evaluator.
/// - [`PowerEvaluator`]: A power evaluator.
/// - [`SigmoidEvaluator`]: A sigmoid evaluator.
/// - [`ExponentialEvaluator`]: An exponential evaluator.
/// - Any [`Fn`] that takes a single `f32` input and returns a `f32` output.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_observed_utility::prelude::*;
/// # use approx::assert_relative_eq;
///
/// # let mut app = App::new();
/// # app.add_plugins(ObservedUtilityPlugins::RealTime);
/// # let mut world = app.world_mut();
/// # let mut commands = world.commands();
/// # let scorer =
/// commands
///     .spawn((Evaluated::new(PowerEvaluator::default()), Score::default()))
///     .with_children(|parent| {
///         parent.spawn((FixedScore::new(0.7), Score::default()));
///     })
/// #   .id();
/// # commands.trigger_targets(RunScoring, scorer);
/// # world.flush();
/// # assert_relative_eq!(world.get::<Score>(scorer).unwrap().get(), 0.49);
/// ```
pub struct Evaluated {
    /// The evaluator to use for scoring.
    evaluator: Box<dyn Evaluator>,
}

impl Evaluated {
    /// Creates a new [`Evaluated`] from the given evaluator.
    #[must_use]
    pub fn new(evaluator: impl Evaluator) -> Self {
        Self {
            evaluator: Box::new(evaluator),
        }
    }

    /// Uses the [`Evaluator`] to evaluate the given value.
    #[must_use]
    pub fn evaluate(&self, value: f32) -> f32 {
        self.evaluator.evaluate(value)
    }

    /// Returns the [`Evaluator`] used for scoring.
    #[must_use]
    pub fn evaluator(&self) -> &dyn Evaluator {
        self.evaluator.as_ref()
    }

    /// Sets the [`Evaluator`] used for scoring.
    pub fn set_evaluator(&mut self, evaluator: impl Evaluator) {
        self.evaluator = Box::new(evaluator);
    }

    /// [`Observer`] for [`Evaluated`] [`Score`] entities that scores a single child [`Score`] entity.
    fn observer(trigger: Trigger<OnScore>, target: Query<(&Children, &Evaluated)>, mut scores: Query<&mut Score>) {
        let Ok((children, settings)) = target.get(trigger.entity()) else {
            // The entity is not scoring for evaluated.
            return;
        };

        if let &[child] = &**children {
            let Ok(child_score) = scores.get_mut(child) else {
                return;
            };
            let value = settings.evaluate(child_score.get());

            let Ok(mut target_score) = scores.get_mut(trigger.entity()) else {
                return;
            };
            target_score.set(value);
        }
    }
}

impl Component for Evaluated {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, _entity, _component| {
            #[derive(Resource, Default)]
            struct EvaluatedObserverSpawned;

            world
                .commands()
                .once::<EvaluatedObserverSpawned>()
                .observe(Self::observer);
        });
    }
}

/// Curves values within a certain range.
#[reflect_trait]
pub trait Evaluator: Send + Sync + 'static {
    /// Evaluates the input value and returns an output value.
    fn evaluate(&self, value: f32) -> f32;
}

/// [`Evaluator`] that uses a linear function to transform a value.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Evaluator, PartialEq, Debug)]
pub struct LinearEvaluator {
    a: Vec2,
    by: f32,
    dy_over_dx: f32,
}

impl LinearEvaluator {
    /// Creates a new linear evaluator with the given parameters.
    #[must_use]
    pub fn new(a: Vec2, b: Vec2) -> Self {
        Self {
            a,
            by: b.y,
            dy_over_dx: (b.y - a.y) / (b.x - a.x),
        }
    }

    /// Creates a linear evaluator with the given range.
    #[must_use]
    pub fn from_range(min: f32, max: f32) -> Self {
        Self::new(Vec2::new(min, 0.), Vec2::new(max, 1.))
    }
}

impl Evaluator for LinearEvaluator {
    fn evaluate(&self, value: f32) -> f32 {
        (self.a.y + self.dy_over_dx * (value - self.a.x)).clamp(self.a.y, self.by)
    }
}

impl Default for LinearEvaluator {
    fn default() -> Self {
        Self::new(Vec2::new(0., 0.), Vec2::new(1., 1.))
    }
}

/// [`Evaluator`] that uses a power function to transform a value.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Evaluator, PartialEq, Debug)]
pub struct PowerEvaluator {
    a: Vec2,
    bx: f32,
    power: f32,
    dy: f32,
}

impl PowerEvaluator {
    /// Creates a new power evaluator with the given parameters.
    #[must_use]
    pub fn new(power: f32, a: Vec2, b: Vec2) -> Self {
        Self {
            power: power.clamp(0., 10000.),
            dy: b.y - a.y,
            a,
            bx: b.x,
        }
    }

    /// Creates a full range power evaluator with the given power value.
    #[must_use]
    pub fn from_power(power: f32) -> Self {
        Self::new(power, Vec2::new(0., 0.), Vec2::new(1., 1.))
    }

    /// Creates a power evaluator with the given power value and the given range.
    #[must_use]
    pub fn from_range(power: f32, min: f32, max: f32) -> Self {
        Self::new(power, Vec2::new(min, 0.), Vec2::new(max, 1.))
    }
}

impl Evaluator for PowerEvaluator {
    fn evaluate(&self, value: f32) -> f32 {
        let cx = value.clamp(self.a.x, self.bx);
        self.dy * ((cx - self.a.x) / (self.bx - self.a.x)).powf(self.power) + self.a.y
    }
}

impl Default for PowerEvaluator {
    fn default() -> Self {
        Self::from_power(2.)
    }
}

/// [`Evaluator`] that uses a sigmoid function to transform a value.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Evaluator, PartialEq, Debug)]
pub struct SigmoidEvaluator {
    a: Vec2,
    b: Vec2,
    k: f32,
    two_over_dx: f32,
    x_mean: f32,
    y_mean: f32,
    dy_over_two: f32,
    one_minus_k: f32,
}

impl SigmoidEvaluator {
    /// Creates a new sigmoid evaluator with the given parameters.
    #[must_use]
    pub fn new(k: f32, a: Vec2, b: Vec2) -> Self {
        let k = k.clamp(-0.99999, 0.99999);
        Self {
            a,
            b,
            two_over_dx: (2. / (b.x - a.x)).abs(),
            x_mean: (a.x + b.x) / 2.,
            y_mean: (a.y + b.y) / 2.,
            dy_over_two: (b.y - a.y) / 2.,
            one_minus_k: 1. - k,
            k,
        }
    }

    /// Creates a full range sigmoid evaluator with the given `k` value.
    #[must_use]
    pub fn from_k(k: f32) -> Self {
        Self::new(k, Vec2::new(0., 0.), Vec2::new(1., 1.))
    }

    /// Creates a sigmoid evaluator with the given `k` value and the given range.
    #[must_use]
    pub fn from_range(k: f32, min: f32, max: f32) -> Self {
        Self::new(k, Vec2::new(min, 0.), Vec2::new(max, 1.))
    }
}

impl Evaluator for SigmoidEvaluator {
    fn evaluate(&self, value: f32) -> f32 {
        let cx_minus_x_mean = value.clamp(self.a.x, self.b.x) - self.x_mean;
        let numerator = self.two_over_dx * cx_minus_x_mean * self.one_minus_k;
        let denominator = 1. + self.k * (1. - 2. * (self.two_over_dx * cx_minus_x_mean)).abs();
        (self.dy_over_two * (numerator / denominator) + self.y_mean).clamp(self.a.y, self.b.y)
    }
}

impl Default for SigmoidEvaluator {
    fn default() -> Self {
        Self::from_k(-0.5)
    }
}

/// [`Evaluator`] that uses an exponential function to transform a value.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Evaluator, PartialEq, Debug)]
pub struct ExponentialEvaluator {
    a: Vec2,
    bx: f32,
    k: f32,
    dy_over_dx: f32,
}

impl ExponentialEvaluator {
    /// Creates a new exponential evaluator with the given parameters.
    #[must_use]
    pub fn new(k: f32, a: Vec2, b: Vec2) -> Self {
        let k = k.clamp(-0.99999, 0.99999);
        Self {
            a,
            bx: b.x,
            k,
            dy_over_dx: (b.y - a.y) / (b.x - a.x),
        }
    }

    /// Creates a full range exponential evaluator with the given `k` value.
    #[must_use]
    pub fn from_k(k: f32) -> Self {
        Self::new(k, Vec2::new(0., 0.), Vec2::new(1., 1.))
    }

    /// Creates an exponential evaluator with the given `k` value and the given range.
    #[must_use]
    pub fn from_range(k: f32, min: f32, max: f32) -> Self {
        Self::new(k, Vec2::new(min, 0.), Vec2::new(max, 1.))
    }
}

impl Evaluator for ExponentialEvaluator {
    fn evaluate(&self, value: f32) -> f32 {
        let cx = value.clamp(self.a.x, self.bx);
        let numerator = (cx - self.a.x) * self.dy_over_dx;
        let denominator = 1. + self.k * (self.bx - cx);
        self.a.y + numerator / denominator
    }
}

impl Default for ExponentialEvaluator {
    fn default() -> Self {
        Self::from_k(-0.5)
    }
}

/// [`Evaluator`] that uses a logarithmic function to transform a value.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Evaluator, PartialEq, Debug)]
pub struct LogarithmicEvaluator {
    a: Vec2,
    bx: f32,
    k: f32,
    dy_over_dx: f32,
}

impl LogarithmicEvaluator {
    /// Creates a new logarithmic evaluator with the given parameters.
    #[must_use]
    pub fn new(k: f32, a: Vec2, b: Vec2) -> Self {
        let k = k.clamp(-0.99999, 0.99999);
        Self {
            a,
            bx: b.x,
            k,
            dy_over_dx: (b.y - a.y) / (b.x - a.x),
        }
    }

    /// Creates a full range logarithmic evaluator with the given `k` value.
    #[must_use]
    pub fn from_k(k: f32) -> Self {
        Self::new(k, Vec2::new(0., 0.), Vec2::new(1., 1.))
    }

    /// Creates a logarithmic evaluator with the given `k` value and the given range.
    #[must_use]
    pub fn from_range(k: f32, min: f32, max: f32) -> Self {
        Self::new(k, Vec2::new(min, 0.), Vec2::new(max, 1.))
    }
}

impl Evaluator for LogarithmicEvaluator {
    fn evaluate(&self, value: f32) -> f32 {
        let cx = value.clamp(self.a.x, self.bx);
        let numerator = (cx - self.a.x) * self.dy_over_dx;
        let denominator = 1. + self.k * (cx - self.a.x);
        self.a.y + numerator / denominator
    }
}

impl<F> Evaluator for F
where
    F: Fn(f32) -> f32 + Send + Sync + 'static,
{
    fn evaluate(&self, value: f32) -> f32 {
        self(value)
    }
}
