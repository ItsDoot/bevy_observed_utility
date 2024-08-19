use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};
use rand::{seq::IteratorRandom, RngCore};

use crate::{
    ecs::{CommandsExt, TriggerGetEntity},
    event::{OnPick, OnPicked},
    picking::Picker,
};

/// [`Picker`] [`Component`] that picks randomly.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_observed_utility::prelude::*;
/// use rand::prelude::{StdRng, SeedableRng};
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
///         PickRandom::new(StdRng::from_entropy()),
///     ))
///     .add_child(scorer)
///     .id();
///
/// commands.trigger_targets(RunScoring, scorer);
/// commands.trigger_targets(RunPicking, actor);
/// # world.flush();
/// # assert_eq!(my_action, world.get::<Picker>(actor).unwrap().picked);
/// ```
pub struct PickRandom {
    /// The random number generator to use.
    pub rng: Box<dyn RngCore + Send + Sync + 'static>,
}

impl PickRandom {
    /// Creates a new [`Random`] with the given random number generator.
    pub fn new(rng: impl RngCore + Send + Sync + 'static) -> Self {
        Self { rng: Box::new(rng) }
    }

    /// Returns a reference to the random number generator.
    pub fn rng(&mut self) -> &mut (impl RngCore + Send + Sync + 'static) {
        &mut self.rng
    }

    /// Sets the random number generator.
    pub fn set_rng(&mut self, rng: impl RngCore + Send + Sync + 'static) {
        self.rng = Box::new(rng);
    }

    /// [`Observer`] for the [`Random`] [`Picker`] that picks randomly.
    fn observer(
        trigger: Trigger<OnPick>,
        mut commands: Commands,
        mut targets: Query<(Entity, &mut Picker, &mut PickRandom)>,
    ) {
        fn run(target: Entity, mut commands: Commands, mut picker: Mut<Picker>, settings: &mut PickRandom) {
            let random = picker.choices.keys().choose(&mut *settings.rng()).copied();
            let action = picker.pick(random);
            commands.trigger_targets(OnPicked { action }, target);
        }

        if let Some(target) = trigger.get_entity() {
            let Ok((target, picker, settings)) = targets.get_mut(target) else {
                return;
            };
            run(target, commands.reborrow(), picker, settings.into_inner());
        } else {
            for (target, picker, settings) in targets.iter_mut() {
                run(target, commands.reborrow(), picker, settings.into_inner());
            }
        }
    }
}

impl<R: RngCore + Send + Sync + 'static> From<R> for PickRandom {
    fn from(rng: R) -> Self {
        Self::new(rng)
    }
}

impl Component for PickRandom {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, _entity, _component| {
            #[derive(Resource, Default)]
            struct RandomObserverSpawned;

            world.commands().once::<RandomObserverSpawned>().observe(Self::observer);
        });
    }
}
