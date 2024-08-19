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

/// [`Picker`] [`Component`] that picks the highest [`Score`](crate::scoring::Score).
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
///         Highest,
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
pub struct Highest;

impl Highest {
    /// [`Observer`] for the [`Highest`] [`Picker`] that picks the highest [`Score`](crate::scoring::Score).
    fn observer(
        trigger: Trigger<OnPick>,
        mut commands: Commands,
        mut targets: Query<(Entity, &Children, &mut Picker), With<Highest>>,
        scores: Query<(Entity, &Score)>,
    ) {
        fn run(
            target: Entity,
            mut commands: Commands,
            children: &Children,
            mut picker: Mut<Picker>,
            scores: &Query<(Entity, &Score)>,
        ) {
            let mut highest_score_entity: Option<(Entity, &Score)> = None;
            for (score_entity, score) in scores.iter_many(children) {
                if let Some((_, highest_score)) = highest_score_entity {
                    if score.get() > highest_score.get() {
                        highest_score_entity = Some((score_entity, score));
                    }
                } else {
                    highest_score_entity = Some((score_entity, score));
                }
            }

            let action = picker.pick(highest_score_entity.map(|(entity, _)| entity));
            commands.trigger_targets(OnPicked { action }, target);
        }

        if let Some(target) = trigger.get_entity() {
            let Ok((target, children, picker)) = targets.get_mut(target) else {
                return;
            };
            run(target, commands.reborrow(), children, picker, &scores);
        } else {
            for (target, children, picker) in targets.iter_mut() {
                run(target, commands.reborrow(), children, picker, &scores);
            }
        }
    }
}

impl Component for Highest {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, _entity, _component| {
            #[derive(Resource, Default)]
            struct HighestObserverSpawned;

            world
                .commands()
                .once::<HighestObserverSpawned>()
                .observe(Self::observer);
        });
    }
}
