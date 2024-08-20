//! Picking in utility AI involves selecting an action based on the scores of child [`Score`] entities.
//!
//! # Provided [`Picker`] implementations
//!
//! - [`FirstToScore`]: Picks the first action to reach a certain score.
//! - [`Highest`]: Picks the action with the highest score.
//! - [`Random`] (requires `rand` feature): Picks a random action.
//!
//! [`Score`]: crate::scoring::Score

use bevy::{
    ecs::{component::ComponentId, entity::EntityHashMap},
    prelude::*,
};

mod first_to_score;
mod highest;
#[cfg(feature = "rand")]
mod random;

pub use first_to_score::*;
pub use highest::*;
#[cfg(feature = "rand")]
pub use random::*;

use crate::{
    ecs::TriggerGetEntity,
    event::{OnPick, RunPicking},
};

/// [`Plugin`] for picking actions based on the scores of child entities.
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::run_picking);
    }
}

impl PickingPlugin {
    /// [`Observer`] that triggers the [`OnPick`] event for one specific or all [`Picker`] entities.
    pub fn run_picking(trigger: Trigger<RunPicking>, mut commands: Commands, pickers: Query<Entity, With<Picker>>) {
        fn trigger_picking(target: Entity, mut commands: Commands) {
            commands.trigger_targets(OnPick, target);
        }

        if let Some(target) = trigger.get_entity() {
            trigger_picking(target, commands.reborrow());
        } else {
            for target in pickers.iter() {
                trigger_picking(target, commands.reborrow());
            }
        }
    }
}

/// [`Component`] for configuring the action to pick based on the scores of child entities.
#[derive(Component, Reflect)]
#[derive(Clone, PartialEq, Debug)]
#[reflect(Component)]
pub struct Picker {
    /// The default action [`ComponentId`] to pick if the picker fails to pick an action.
    pub default: ComponentId,
    /// Map of child [`Score`](crate::scoring::Score) [`Entity`]s to action [`ComponentId`]s.
    pub choices: EntityHashMap<ComponentId>,
    /// The last action [`ComponentId`] picked by the picker.
    pub picked: ComponentId,
}

impl Picker {
    /// Creates a new [`Picker`] with the given default action [`ComponentId`].
    #[must_use]
    pub fn new(default: ComponentId) -> Self {
        Self {
            default,
            choices: EntityHashMap::default(),
            picked: default,
        }
    }

    /// Adds an action [`ComponentId`] to pick based on the provided score [`Entity`].
    #[must_use]
    pub fn with(mut self, score_entity: Entity, action: ComponentId) -> Self {
        self.choices.insert(score_entity, action);
        self
    }

    /// Grab the action [`ComponentId`] to pick based on the score [`Entity`] and the picker's choices.
    pub fn pick(&mut self, score_entity: Option<Entity>) -> ComponentId {
        let action = score_entity
            .and_then(|entity| self.choices.get(&entity).copied())
            .unwrap_or(self.default);
        self.picked = action;
        action
    }

    /// Returns `true` if the given action is the default action.
    #[must_use]
    pub fn is_default(&self, action: ComponentId) -> bool {
        action == self.default
    }

    /// Returs `true` if the last picked action is the default action.
    #[must_use]
    pub fn picked_default(&self) -> bool {
        self.picked == self.default
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::{
        event::{RunPicking, RunScoring},
        picking::{FirstToScore, Highest, Picker},
        scoring::{FixedScore, Score},
    };

    #[derive(Component)]
    struct MyAction;

    #[derive(Component)]
    struct IdleAction;

    #[test]
    fn pick_first_to_score() {
        let mut app = App::new();
        app.add_plugins(crate::ObservedUtilityPlugins::RealTime);
        let world = app.world_mut();

        let my_action = world.init_component::<MyAction>();
        let idle_action = world.init_component::<IdleAction>();

        let mut commands = world.commands();

        let scorer = commands.spawn((FixedScore::new(0.7), Score::default())).id();
        let actor = commands
            .spawn((Picker::new(idle_action).with(scorer, my_action), FirstToScore::new(0.5)))
            .add_child(scorer)
            .id();

        commands.trigger_targets(RunScoring, scorer);
        commands.trigger_targets(RunPicking, actor);
        world.flush();

        assert_eq!(my_action, world.get::<Picker>(actor).unwrap().picked);
    }

    #[test]
    fn pick_highest() {
        let mut app = App::new();
        app.add_plugins(crate::ObservedUtilityPlugins::RealTime);
        let world = app.world_mut();

        let my_action = world.init_component::<MyAction>();
        let idle_action = world.init_component::<IdleAction>();

        let mut commands = world.commands();

        let scorer = commands.spawn((FixedScore::new(0.7), Score::default())).id();
        let actor = commands
            .spawn((Picker::new(idle_action).with(scorer, my_action), Highest))
            .add_child(scorer)
            .id();

        commands.trigger_targets(RunScoring, scorer);
        commands.trigger_targets(RunPicking, actor);
        world.flush();

        assert_eq!(my_action, world.get::<Picker>(actor).unwrap().picked);
    }
}
