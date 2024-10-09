//! Scoring in utility AI involves calculating the [`Score`] of an entity, often based on the scores of its children.
//!
//! # Provided [`Score`] implementations
//!
//! - [`AllOrNothing`]: Scores the sum of all child scores, but only if the sum reaches a certain threshold. Otherwise, the score is 0.
//! - [`Evaluated`]: Scores a single child entity based on an [`Evaluator`] function. See the struct docs for the list of provided evaluators.
//! - [`FixedScore`]: Scores a fixed value.
//! - [`Measured`]: Scores all child entities based on a [`Measure`] function. See the struct docs for the list of provided measures.
//! - [`Product`]: Scores the product of all child scores.
//! - [`Random`] (requires `rand` feature): Scores a random value, optionally within a range.
//! - [`Sum`]: Scores the sum of all child scores.
//! - [`Winning`]: Scores the highest child score.
//!
//! # Provided [`Observer`] utilities
//!
//! - [`score_ancestor`]: Does the busy work of scoring a child entity based on its closest ancestor entity with a given component.

use std::{
    cmp::Ordering,
    ops::{Bound, RangeBounds},
};

use bevy::prelude::*;

use crate::{
    ecs::{AncestorQuery, DFSPostTraversal, TriggerGetEntity},
    event::{OnScore, RunScoring},
};

mod all_or_nothing;
mod evaluator;
mod fixed;
mod measured;
mod product;
#[cfg(feature = "rand")]
mod random;
mod sum;
mod winning;

pub use self::all_or_nothing::*;
pub use self::evaluator::*;
pub use self::fixed::*;
pub use self::measured::*;
pub use self::product::*;
#[cfg(feature = "rand")]
pub use self::random::*;
pub use self::sum::*;
pub use self::winning::*;

/// [`Plugin`] for scoring entities.
#[derive(Default)]
pub struct ScoringPlugin;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::run_scoring_post_order_dfs);

        app.register_type::<Score>()
            .register_type::<AllOrNothing>()
            // .register_type::<Evaluated>() // TODO: Implement reflection for Evaluated
            .register_type::<LinearEvaluator>()
            .register_type::<PowerEvaluator>()
            .register_type::<SigmoidEvaluator>()
            .register_type::<ExponentialEvaluator>()
            .register_type::<LogarithmicEvaluator>()
            .register_type::<FixedScore>()
            // .register_type::<Measured>() // TODO: Implement reflection for Measured
            .register_type::<Weighted>()
            .register_type::<WeightedSum>()
            .register_type::<WeightedProduct>()
            .register_type::<WeightedMax>()
            .register_type::<WeightedRMS>()
            .register_type::<Product>()
            .register_type::<Sum>()
            .register_type::<Winning>();

        #[cfg(feature = "rand")]
        app.register_type::<RandomScore>();

        app.register_type::<RunScoring>().register_type::<OnScore>();
    }
}

impl ScoringPlugin {
    /// For each scoreable root entity, perform post-order depth-first traversal,
    /// triggering [`OnScore`] for each entity on the way back up.
    pub fn run_scoring_post_order_dfs(
        trigger: Trigger<RunScoring>,
        mut commands: Commands,
        scoreable_roots: Query<(Entity, Option<&Parent>), With<Score>>,
        root_parents: Query<(), Without<Score>>,
        mut dfs: DFSPostTraversal<With<Score>>,
    ) {
        fn trigger_in_order(root: Entity, mut commands: Commands, dfs: &mut DFSPostTraversal<With<Score>>) {
            let sorted = dfs.iter(root);

            for entity in sorted {
                commands.trigger_targets(OnScore, entity);
            }
        }

        if let Some(targeted_root) = trigger.get_entity() {
            // Do scoring for the given entity
            trigger_in_order(targeted_root, commands.reborrow(), &mut dfs);
        } else {
            // Do scoring globally
            // Find all score entities that have no parents at all, or whose parents are not score entities
            let roots = scoreable_roots.iter().filter_map(|(entity, parent)| {
                if let Some(parent) = parent {
                    if root_parents.contains(**parent) {
                        Some(entity)
                    } else {
                        None
                    }
                } else {
                    Some(entity)
                }
            });
            for root in roots {
                trigger_in_order(root, commands.reborrow(), &mut dfs);
            }
        }
    }
}

/// [`Component`] for an entity's score for a given score type, ranging from 0 to 1.
#[derive(Component, Reflect)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct Score {
    /// The score value, clamped to the range `[0, 1]`.
    value: f32,
}

impl Score {
    /// The minimum possible score.
    // SAFETY: The value is within the valid range of `[0, 1]`.
    pub const MIN: Score = unsafe { Score::new_unchecked(0.) };
    /// The maximum possible score.
    // SAFETY: The value is within the valid range of `[0, 1]`.
    pub const MAX: Score = unsafe { Score::new_unchecked(1.) };

    /// Creates a new score with the given value, clamped to the range `[0, 1]`.
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0., 1.),
        }
    }

    /// Creates a new score with the given value, without clamping.
    ///
    /// # Safety
    ///
    /// The value must be in the range `[0, 1]`.
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Self { value }
    }

    /// Returns the score's value.
    #[must_use]
    pub fn get(&self) -> f32 {
        self.value
    }

    /// Sets the score's value, clamped to the range `[0, 1]`.
    #[inline]
    pub fn set(&mut self, value: f32) {
        self.value = value.clamp(0., 1.);
    }
}

impl From<f32> for Score {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<Score> for f32 {
    fn from(score: Score) -> f32 {
        score.get()
    }
}

impl PartialEq<f32> for Score {
    fn eq(&self, other: &f32) -> bool {
        self.get() == *other
    }
}

impl PartialEq<Score> for f32 {
    fn eq(&self, other: &Score) -> bool {
        *self == other.get()
    }
}

impl PartialOrd<f32> for Score {
    fn partial_cmp(&self, other: &f32) -> Option<Ordering> {
        self.get().partial_cmp(other)
    }
}

impl PartialOrd<Score> for f32 {
    fn partial_cmp(&self, other: &Score) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl std::iter::Sum<Self> for Score {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        // Unwrap the values, sum them, and clamp the result to the range `[0, 1]`.
        iter.map(|v| v.get())
            .fold(Score::MIN.get(), |acc, score| acc + score)
            .into()
    }
}

impl std::iter::Sum<f32> for Score {
    fn sum<I: Iterator<Item = f32>>(iter: I) -> Self {
        // Sum the values, and clamp the result to the range `[0, 1]`.
        iter.fold(Score::MIN.get(), |acc, score| acc + score).into()
    }
}

impl std::iter::Sum<Score> for f32 {
    fn sum<I: Iterator<Item = Score>>(iter: I) -> Self {
        // Unwrap the values, sum them.
        let sum = iter.map(|v| v.get()).fold(Score::MIN.get(), |acc, score| acc + score);
        // Clamp the result to the range `[0, 1]`.
        Score::new(sum).get()
    }
}

// TODO: implement Reflect when Bound is reflectable
/// A range of [`Score`]s.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ScoreRange {
    /// The minimum score.
    min: Bound<Score>,
    /// The maximum score.
    max: Bound<Score>,
}

impl ScoreRange {
    /// The full range of scores, from 0 to 1.
    pub const FULL: ScoreRange = ScoreRange {
        min: Bound::Included(Score::MIN),
        max: Bound::Included(Score::MAX),
    };

    /// Creates a new score range with the given minimum and maximum scores.
    #[must_use]
    pub fn new(mut min: Bound<Score>, mut max: Bound<Score>) -> Self {
        match (&mut min, &mut max) {
            (Bound::Included(min) | Bound::Excluded(min), Bound::Included(max) | Bound::Excluded(max))
                if *max < *min =>
            {
                std::mem::swap(min, max);
            }
            _ => {}
        };
        Self { min, max }
    }

    /// Creates a new score range from the given [`RangeBounds`].
    #[must_use]
    pub fn from_bounds(bounds: impl RangeBounds<Score>) -> Self {
        Self::new(bounds.start_bound().cloned(), bounds.end_bound().cloned())
    }

    /// Returns the minimum score.
    #[must_use]
    pub fn min(&self) -> Bound<Score> {
        self.min
    }

    /// Returns the minimum score as a `f32`.
    #[must_use]
    pub fn min_f32(&self) -> f32 {
        match self.min {
            Bound::Included(score) | Bound::Excluded(score) => score.get(),
            Bound::Unbounded => Score::MIN.get(),
        }
    }

    /// Returns the maximum score.
    #[must_use]
    pub fn max(&self) -> Bound<Score> {
        self.max
    }

    /// Returns the maximum score as a `f32`.
    #[must_use]
    pub fn max_f32(&self) -> f32 {
        match self.max {
            Bound::Included(score) | Bound::Excluded(score) => score.get(),
            Bound::Unbounded => Score::MAX.get(),
        }
    }
}

impl Default for ScoreRange {
    fn default() -> Self {
        Self::FULL
    }
}

impl RangeBounds<Score> for ScoreRange {
    fn start_bound(&self) -> Bound<&Score> {
        self.min.as_ref()
    }

    fn end_bound(&self) -> Bound<&Score> {
        self.max.as_ref()
    }
}

/// [`Observer`] helper function that calculates the score of a child [`Score`] entity marked with `ScoreMarker`
/// based on the [`Component`] `T` on its closest ancestor entity, usually the actor entity.
///
/// The [`Component`] `T` must implement [`Into<Score>`] for its reference type `&T`.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_observed_utility::prelude::*;
///
/// /// This goes on the actor entity.
/// #[derive(Component)]
/// struct Thirst {
///     value: f32,
///     per_second: f32,
/// }
///
/// /// This impl is required for the `score_ancestor` observer.
/// impl From<&Thirst> for Score {
///    fn from(thirst: &Thirst) -> Self {
///       Score::new(thirst.value / 100.)
///    }
/// }
///
/// /// This goes on the score entity.
/// #[derive(Component)]
/// pub struct Thirsty;
///
/// # let mut app = App::new();
/// # app.add_plugins(ObservedUtilityPlugins::RealTime);
/// app.observe(score_ancestor::<Thirst, Thirsty>);
///
/// # let mut world = app.world_mut();
/// # let mut commands = world.commands();
/// let scorer = commands
///     .spawn((Thirsty, Score::default()))
///     .id();
///
/// let actor = commands
///     .spawn(Thirst { value: 50., per_second: 1. })
///     .add_child(scorer)
///     .id();
/// # commands.trigger_targets(RunScoring, scorer);
/// # world.flush();
/// # assert_eq!(0.5, world.get::<Score>(scorer).unwrap().get());
/// ```
pub fn score_ancestor<T: Component, ScoreMarker: Component>(
    trigger: Trigger<OnScore>,
    mut scores: Query<&mut Score, With<ScoreMarker>>,
    mut ancestors: AncestorQuery<&'static T>,
) where
    for<'a> &'a T: Into<Score>,
{
    let scorer = trigger.entity();
    let Ok(mut score) = scores.get_mut(scorer) else {
        return;
    };

    if let Ok(ancestor) = ancestors.get(scorer) {
        *score = ancestor.into();
    } else {
        // If there is no ancestor, set the score to the minimum.
        *score = Score::MIN;
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use bevy::{
        app::App,
        ecs::observer::ObserverState,
        prelude::{BuildWorldChildren, With, World},
    };

    use crate::{
        event::RunScoring,
        scoring::{
            AllOrNothing, Evaluated, FixedScore, Measured, PowerEvaluator, Product, Score, ScoringPlugin, Sum,
            Weighted, WeightedMax, WeightedProduct, WeightedRMS, WeightedSum, Winning,
        },
    };

    #[test]
    fn all_or_nothing() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), AllOrNothing::new(0.2)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.7)));
                parent.spawn((Score::default(), FixedScore::new(0.3)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_eq!(
            1.0,
            world.get::<Score>(parent).unwrap().get(),
            "Parent score should be 1.0."
        );
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn evaluated_power() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let entity = world
            .spawn((Score::default(), Evaluated::new(PowerEvaluator::default())))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.7)));
            })
            .id();

        world.trigger_targets(RunScoring, entity);
        world.flush();

        assert_relative_eq!(0.49, world.get::<Score>(entity).unwrap().get());
    }

    #[test]
    fn fixed() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let entity = world.spawn((Score::default(), FixedScore::new(0.5))).id();

        world.trigger_targets(RunScoring, entity);
        world.flush();

        assert_eq!(0.5, world.get::<Score>(entity).unwrap().get(), "Score should be 0.5.");
        assert_eq!(2, count_observers(world));
    }

    #[test]
    fn measured_weighted_sum() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Measured::new(WeightedSum)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9), Weighted::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8), Weighted::new(0.1)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_relative_eq!(0.89, world.get::<Score>(parent).unwrap().get());
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn measured_weighted_product() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Measured::new(WeightedProduct)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9), Weighted::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8), Weighted::new(0.1)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_relative_eq!(0.0648, world.get::<Score>(parent).unwrap().get());
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn measured_weighted_max() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Measured::new(WeightedMax)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9), Weighted::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8), Weighted::new(0.1)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_relative_eq!(0.81, world.get::<Score>(parent).unwrap().get());
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn measured_weighted_rms() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Measured::new(WeightedRMS)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9), Weighted::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8), Weighted::new(0.1)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_relative_eq!(0.8905055, world.get::<Score>(parent).unwrap().get());
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn product() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Product::new(0.4)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_relative_eq!(0.72, world.get::<Score>(parent).unwrap().get(),);
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn sum() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Sum::new(0.4)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_eq!(
            1.0,
            world.get::<Score>(parent).unwrap().get(),
            "Parent score should be 1.0."
        );
        assert_eq!(3, count_observers(world));
    }

    #[test]
    fn winning() {
        let mut app = App::new();
        app.add_plugins(ScoringPlugin);

        let world = app.world_mut();

        let parent = world
            .spawn((Score::default(), Winning::new(0.5)))
            .with_children(|parent| {
                parent.spawn((Score::default(), FixedScore::new(0.9)));
                parent.spawn((Score::default(), FixedScore::new(0.8)));
            })
            .id();

        world.trigger_targets(RunScoring, parent);
        world.flush();

        assert_eq!(
            0.9,
            world.get::<Score>(parent).unwrap().get(),
            "Parent score should be 0.9."
        );
        assert_eq!(3, count_observers(world));
    }

    fn count_observers(world: &mut World) -> usize {
        world.query_filtered::<(), With<ObserverState>>().iter(world).count()
    }
}
