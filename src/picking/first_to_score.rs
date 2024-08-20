use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};

use crate::{
    ecs::{CommandsExt, TriggerGetEntity},
    event::{OnPick, OnPicked},
    picking::Picker,
    scoring::Score,
};

/// [`Picker`] [`Component`] that picks the first [`Score`] entity to reach a certain threshold.
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
/// #[derive(Component)]
/// pub struct MyAction;
/// #[derive(Component)]
/// pub struct IdleAction;
///
/// // We need the ComponentIds to initialize the Picker.
/// // It's recommended to use a resource to store these.
/// let my_action = world.init_component::<MyAction>();
/// let idle_action = world.init_component::<IdleAction>();
///
/// # let mut commands = world.commands();
/// // Spawn the scorer entity that will be picked by the actor.
/// let scorer = commands
///     .spawn((FixedScore::new(0.7), Score::default()))
///     .id();
///
/// // Spawn the actor entity that will pick an action based on all of its children scores.
/// let actor = commands
///     .spawn((
///         // All pickers need a default action to pick if they fail to pick an action.
///         Picker::new(idle_action)
///             // if the score entity is selected, my_action will be picked.
///             .with(scorer, my_action),
///         FirstToScore::new(0.5),
///     ))
///     .add_child(scorer)
///     .id();
///
/// commands.trigger_targets(RunScoring, scorer);
/// commands.trigger_targets(RunPicking, actor);
/// # world.flush();
/// # assert_eq!(my_action, world.get::<Picker>(actor).unwrap().picked);
/// ```
#[derive(Reflect)]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
#[reflect(Component)]
pub struct FirstToScore {
    /// The [`Score`] threshold for the entity to be picked.
    threshold: Score,
}

impl FirstToScore {
    /// Creates a new [`FirstToScore`] with the given threshold.
    #[must_use]
    pub fn new(threshold: impl Into<Score>) -> Self {
        Self {
            threshold: threshold.into(),
        }
    }

    /// Returns the threshold for the entity to be picked.
    #[must_use]
    pub fn threshold(&self) -> Score {
        self.threshold
    }

    /// Sets the threshold for the entity to be picked.
    pub fn set_threshold(&mut self, threshold: impl Into<Score>) {
        self.threshold = threshold.into();
    }

    /// [`Observer`] for the [`FirstToScore`] [`Picker`] that picks the first entity to reach a certain
    /// [`Score`](crate::scoring::Score) threshold.
    fn observer(
        trigger: Trigger<OnPick>,
        mut commands: Commands,
        mut targets: Query<(Entity, &Children, &mut Picker, &FirstToScore)>,
        scores: Query<(Entity, &Score)>,
    ) {
        fn run(
            target: Entity,
            mut commands: Commands,
            children: &Children,
            mut picker: Mut<Picker>,
            settings: &FirstToScore,
            scores: &Query<(Entity, &Score)>,
        ) {
            for (score_entity, score) in scores.iter_many(children) {
                if *score >= settings.threshold() {
                    picker.pick(Some(score_entity));
                    return;
                }
            }

            // If no score entity reached the threshold, pick the default action
            let action = picker.pick(None);
            commands.trigger_targets(OnPicked { action }, target);
        }

        if let Some(target) = trigger.get_entity() {
            let Ok((target, children, picker, settings)) = targets.get_mut(target) else {
                return;
            };
            run(target, commands.reborrow(), children, picker, settings, &scores);
        } else {
            for (target, children, picker, settings) in &mut targets {
                run(target, commands.reborrow(), children, picker, settings, &scores);
            }
        }
    }
}

impl Component for FirstToScore {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, _entity, _component| {
            #[derive(Resource, Default)]
            struct FirstToScoreObserverSpawned;

            world
                .commands()
                .once::<FirstToScoreObserverSpawned>()
                .observe(Self::observer);
        });
    }
}
