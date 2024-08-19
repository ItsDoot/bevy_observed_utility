use bevy::{ecs::component::ComponentId, log::LogPlugin, prelude::*};
use bevy_observed_utility::prelude::*;

#[derive(Component)]
pub struct Thirst {
    /// From 0 to 100.
    pub value: f32,
    /// From 0 to 100.
    pub per_second: f32,
}

/// This impl allows us to use [`score_ancestor`] to score thirst.
impl From<&Thirst> for Score {
    fn from(thirst: &Thirst) -> Self {
        Score::new(thirst.value / 100.)
    }
}

pub fn get_thirsty_over_time(time: Res<Time<Fixed>>, mut thirsts: Query<&mut Thirst>) {
    for mut thirst in thirsts.iter_mut() {
        thirst.value = (thirst.value + thirst.per_second * time.delta_seconds()).min(100.);
        info!("Thirst: {}", thirst.value);
    }
}

#[derive(Component)]
pub struct Thirsty;

pub fn spawn_entities(mut commands: Commands, actions: Res<ActionIds>) {
    let thirst = commands.spawn((Thirsty, Score::default())).id();

    commands
        .spawn((
            Name::new("Actor"),
            Picker::new(actions.idle).with(thirst, actions.drink),
            Thirst {
                value: 0.,
                per_second: 4.,
            },
            FirstToScore::new(0.5),
        ))
        .add_child(thirst);
}

#[derive(Component, Resource, Reflect)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Drinking {
    pub until: f32,
    pub per_second: f32,
}

impl Default for Drinking {
    fn default() -> Self {
        Self {
            until: 10.,
            per_second: 8.,
        }
    }
}

pub fn quench_thirst(
    mut commands: Commands,
    actions: Res<ActionIds>,
    time: Res<Time<Fixed>>,
    mut drinking: Query<(Entity, &mut Thirst, &Drinking)>,
) {
    for (actor, mut thirst, drink) in drinking.iter_mut() {
        thirst.value = (thirst.value - drink.per_second * time.delta_seconds()).max(0.);
        info!("DRINKING!");
        if thirst.value <= drink.until {
            commands.trigger_targets(
                OnActionEnded {
                    action: actions.drink,
                    reason: ActionEndReason::Completed,
                },
                TargetedAction(actor, actions.drink),
            )
        }
    }
}

#[derive(Component, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Idle;

#[derive(Resource)]
pub struct ActionIds {
    drink: ComponentId,
    idle: ComponentId,
}

impl FromWorld for ActionIds {
    fn from_world(world: &mut World) -> Self {
        Self {
            drink: world.init_component::<Drinking>(),
            idle: world.init_component::<Idle>(),
        }
    }
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin {
            filter: "thirst=debug".to_string(),
            ..default()
        })
        .add_plugins(ObservedUtilityPlugins::RealTime)
        .init_resource::<ActionIds>()
        .init_resource::<Drinking>()
        .add_systems(Startup, spawn_entities)
        .add_systems(FixedUpdate, (get_thirsty_over_time, quench_thirst).chain())
        .observe(score_ancestor::<Thirst, Thirsty>)
        .observe(on_action_initiated_insert_from_resource::<Drinking>)
        .observe(on_action_ended_remove::<Drinking>)
        .run();
}
