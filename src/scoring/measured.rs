use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};

use crate::{ecs::CommandsExt, event::OnScore, scoring::Score};

/// [`Score`] [`Component`] that scores based on a [`Measure`] of its child [`Score`] + [`Weighted`] entities.
/// Child entities without a [`Weighted`] component are considered fully weighted (1.0).
///
/// # Provided [`Measure`]s
///
/// - [`WeightedSum`]: The sum of the weighted input scores.
/// - [`WeightedProduct`]: The product of the weighted input scores.
/// - [`WeightedMax`]: The max of the weighted input scores.
/// - [`WeightedRMS`]: The root mean square of the weighted input scores.
/// - Any [`Fn`] that takes a [`Vec<(&Score, &Weighted)>`] input and returns a [`Score`] output.
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
///     .spawn((Measured::new(WeightedSum), Score::default()))
///     .with_children(|parent| {
///         parent.spawn((Weighted::new(0.9), FixedScore::new(0.9), Score::default()));
///         parent.spawn((Weighted::new(0.1), FixedScore::new(0.8), Score::default()));
///     })
/// #   .id();
/// # commands.trigger_targets(RunScoring, scorer);
/// # world.flush();
/// # assert_relative_eq!(world.get::<Score>(scorer).unwrap().get(), 0.89);
/// ```
pub struct Measured {
    /// The function that calculates the score.
    measure: Box<dyn Measure>,
}

impl Measured {
    /// Creates a new measured score from the given function.
    #[must_use]
    pub fn new(measure: impl Measure) -> Self {
        Self {
            measure: Box::new(measure),
        }
    }

    /// Uses the [`Measure`] to calculate the output score based on the input scores and weights.
    #[must_use]
    pub fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score {
        self.measure.calculate(inputs)
    }

    /// Returns the [`Measure`] used for scoring.
    #[must_use]
    pub fn measure(&self) -> &dyn Measure {
        self.measure.as_ref()
    }

    /// Sets the [`Measure`] used for scoring.
    pub fn set_measure(&mut self, measure: impl Measure) {
        self.measure = Box::new(measure);
    }

    /// [`Observer`] for [`Measured`] [`Score`] entities that scores based on all child [`Score`] entities.
    fn observer(
        trigger: Trigger<OnScore>,
        target: Query<(&Children, &Measured)>,
        mut scores: Query<(&mut Score, Option<&Weighted>)>,
    ) {
        let Ok((children, settings)) = target.get(trigger.entity()) else {
            // The entity is not scoring for measured.
            return;
        };

        let mut inputs = Vec::new();

        for (child_score, weighted) in scores.iter_many(children) {
            inputs.push((child_score, weighted.unwrap_or(&Weighted::MAX)));
        }

        let result = settings.calculate(inputs);

        let Ok((mut actor_score, _)) = scores.get_mut(trigger.entity()) else {
            // The entity is not scoring.
            return;
        };

        *actor_score = result;
    }
}

impl Component for Measured {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, _entity, _component| {
            #[derive(Resource, Default)]
            struct MeasuredObserverSpawned;

            world
                .commands()
                .once::<MeasuredObserverSpawned>()
                .observe(Self::observer);
        });
    }
}

/// [`Score`] [`Component`] that's added to each child [`Score`] entity to weight it in the [`Measure`].
///
/// See [`Measured`] for more information.
#[derive(Component, Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct Weighted {
    /// The weight of a score, clamped to the range `[0, 1]`.
    weight: Score,
}

impl Default for Weighted {
    fn default() -> Self {
        Self::MAX
    }
}

impl Weighted {
    /// The minimum weight of 0 (i.e. not considered).
    pub const MIN: Weighted = Weighted {
        // SAFETY: The value is within the valid range of `[0, 1]`.
        weight: unsafe { Score::new_unchecked(0.) },
    };

    /// The default and maximum weight of 1 (i.e. fully considered).
    pub const MAX: Weighted = Weighted {
        // SAFETY: The value is within the valid range of `[0, 1]`.
        weight: unsafe { Score::new_unchecked(1.) },
    };

    /// Creates a new scoring weight, clamped to the range `[0, 1]`.
    #[must_use]
    pub fn new(weight: impl Into<Score>) -> Self {
        Self { weight: weight.into() }
    }

    /// Returns the weight.
    #[must_use]
    pub fn get(&self) -> Score {
        self.weight
    }

    /// Sets the weight, clamped to the range `[0, 1]`.
    pub fn set(&mut self, weight: impl Into<Score>) {
        self.weight = weight.into();
    }
}

/// A measure of scoring.
#[reflect_trait]
pub trait Measure: Send + Sync + 'static {
    /// Calculates the output score based on the input scores and weights.
    fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score;
}

/// [`Measure`] that calculates the sum of the weighted input scores.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Measure, PartialEq, Debug)]
pub struct WeightedSum;

impl Measure for WeightedSum {
    fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score {
        let sum = inputs
            .iter()
            .fold(0., |acc, (score, weight)| acc + score.get() * weight.get().get());
        Score::new(sum)
    }
}

/// [`Measure`] that calculates the product of the weighted input scores.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Measure, PartialEq, Debug)]
pub struct WeightedProduct;

impl Measure for WeightedProduct {
    fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score {
        let product = inputs
            .iter()
            .fold(1., |acc, (score, weight)| acc * score.get() * weight.get().get());
        Score::new(product)
    }
}

/// [`Measure`] that calculates the max of the weighted input scores.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Measure, PartialEq, Debug)]
pub struct WeightedMax;

impl Measure for WeightedMax {
    fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score {
        let max = inputs
            .iter()
            .fold(0., |best, (score, weight)| (score.get() * weight.get().get()).max(best));
        Score::new(max)
    }
}

/// [`Measure`] that calculates the root mean square of the input scores.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Measure, PartialEq, Debug)]
pub struct WeightedRMS;

impl Measure for WeightedRMS {
    fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score {
        let weight_sum = inputs.iter().map(|(_, weight)| weight.get()).sum::<f32>();

        if weight_sum == 0. {
            Score::MIN
        } else {
            let rms = inputs
                .iter()
                .map(|(score, weight)| weight.get().get() / weight_sum * score.get().powf(2.))
                .sum::<f32>()
                .sqrt();
            Score::new(rms)
        }
    }
}

impl<F> Measure for F
where
    F: Fn(Vec<(&Score, &Weighted)>) -> Score + Send + Sync + 'static,
{
    fn calculate(&self, inputs: Vec<(&Score, &Weighted)>) -> Score {
        self(inputs)
    }
}
