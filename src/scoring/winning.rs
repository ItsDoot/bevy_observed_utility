use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};

use crate::{ecs::CommandsExt, event::OnScore, scoring::Score};

/// [`Score`] [`Component`] that scores based on the maximum of its child [`Score`] entities.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_observed_utility::prelude::*;
///
/// # let mut app = App::new();
/// # app.add_plugins(ObservedUtilityPlugins::RealTime);
/// # let mut world = app.world_mut();
/// # let mut commands = world.commands();
/// # let scorer =
/// commands
///     .spawn((Winning::new(0.5), Score::default()))
///     .with_children(|parent| {
///         parent.spawn((FixedScore::new(0.7), Score::default()));
///         parent.spawn((FixedScore::new(0.3), Score::default()));
///     })
/// #   .id();
/// # commands.trigger_targets(RunScoring, scorer);
/// # world.flush();
/// # assert_eq!(world.get::<Score>(scorer).unwrap().get(), 0.7);
/// ```
#[derive(Reflect, Clone, Copy, PartialEq, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct Winning {
    /// The threshold for the maximum of child scores to be considered a success.
    threshold: Score,
}

impl Winning {
    /// Creates a new [`Winning`] with the given threshold.
    #[must_use]
    pub fn new(threshold: impl Into<Score>) -> Self {
        Self {
            threshold: threshold.into(),
        }
    }

    /// Returns the threshold for the maximum of child scores to be considered a success.
    #[must_use]
    pub fn threshold(&self) -> Score {
        self.threshold
    }

    /// Sets the threshold for the maximum of child scores to be considered a success.
    pub fn set_threshold(&mut self, threshold: Score) {
        self.threshold = threshold;
    }

    /// [`Observer`] for [`Winning`] [`Score`] entities that scores based on all child [`Score`] entities.
    fn observer(trigger: Trigger<OnScore>, actor: Query<(&Children, &Winning)>, mut scores: Query<&mut Score>) {
        let Ok((children, settings)) = actor.get(trigger.entity()) else {
            // The entity is not scoring for winning.
            return;
        };

        let mut max: f32 = 0.;

        for child_score in scores.iter_many(children) {
            if child_score.get() > max {
                max = child_score.get();
            }
        }
        if max < settings.threshold().get() {
            max = 0.;
        }

        let Ok(mut actor_score) = scores.get_mut(trigger.entity()) else {
            // The entity is not scoring.
            return;
        };

        actor_score.set(max);
    }
}

impl Component for Winning {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, _entity, _component| {
            #[derive(Resource, Default)]
            struct WinningObserverSpawned;

            world
                .commands()
                .once::<WinningObserverSpawned>()
                .observe(Self::observer);
        });
    }
}
