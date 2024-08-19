//! Acting in utility AI is where the actors perform their actions, based on what their [`Picker`] has picked.
//!
//! This library intentionally provides little infrastructure for acting, as it tends to be highly dependent on the game's mechanics.
//!
//! However, the library does provide these types:
//! - [`RequestAction`] event to request a specific action or the picked action to be initiated for the target actor entity.
//! - [`OnActionInitiated`] event to indicate that an action has been initiated. This should be listened to by action observers.
//! - [`OnActionEnded`] event to indicate that an action has completed or been cancelled. This should be listened to by action observers.
//! - [`CurrentAction`] component to store the current action being performed by an actor entity, for easy access.
//!
//! And, these observers:
//! - [`on_action_initiated_insert_default`] to insert a default instance of an action component when it is initiated.
//!     - This can then be queried with a [`With<ActionT>`] query by action systems.
//! - [`on_action_initiated_insert_from_resource`] to insert a clone of an action component from a resource when it is initiated.
//!     - Same as above, but with a resource as the source.
//! - [`on_action_ended_remove`] to remove an action component when it is ended.

use bevy::{ecs::component::ComponentId, prelude::*};

use crate::{
    ecs::TargetedAction,
    event::{ActionEndReason, OnActionEnded, OnActionInitiated, RequestAction},
    picking::Picker,
};

/// [`Plugin`] that handles action lifecycle events.
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::on_request_cancel_and_initiate)
            .observe(Self::on_ended_request_again);
    }
}

impl ActionPlugin {
    /// [`System`] that listens for [`RequestAction`] events and cancels the current action
    /// and initiates the picked action for the target actor entity.
    pub fn on_request_cancel_and_initiate(
        trigger: Trigger<RequestAction>,
        mut commands: Commands,
        mut actors: Query<(&Picker, Option<&CurrentAction>)>,
    ) {
        let actor = trigger.entity();
        let requested = trigger.event().action;
        if let Ok((picker, current_action)) = actors.get_mut(actor) {
            let current_action = current_action.map(|ca| ca.0);
            let next_action = requested.unwrap_or(picker.picked);

            if let Some(current_action) = current_action {
                if next_action == current_action {
                    // We don't need to re-initiate the same action
                    return;
                }

                // Cancel the current action
                commands.trigger_targets(
                    OnActionEnded::cancelled(current_action),
                    TargetedAction(actor, current_action),
                );
            }

            // Update the current action
            commands.entity(actor).insert(CurrentAction(next_action));
            // Trigger the picked action
            commands.trigger_targets(
                OnActionInitiated { action: next_action },
                TargetedAction(actor, next_action),
            );
        }
    }

    /// [`Observer`] that listens for [`OnActionEnded`] events and triggers a new [`RequestAction`] event for the target actor entity.
    pub fn on_ended_request_again(trigger: Trigger<OnActionEnded>, mut commands: Commands) {
        let actor = trigger.entity();

        match trigger.event().reason {
            ActionEndReason::Completed => {
                // Pick a new action
                commands.trigger_targets(RequestAction { action: None }, actor);
            }
            ActionEndReason::Cancelled => {
                // Do nothing
            }
        }
    }
}

/// [`Component`] for the current action picked by a [`Picker`].
///
/// This component is used by the [`ActionPlugin`] when switching actions so that
/// the previous action can be cancelled before the new action is initiated.
///
/// [`Picker`]: crate::picking::Picker
#[derive(Component, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[reflect(Component)]
pub struct CurrentAction(pub ComponentId);

/// [`Observer`] that listens for [`OnActionInitiated`] events targeting
/// the specified `Action` [`Component`] and inserts a [`Default`] instance of it
/// onto the actor entity.
///
/// Alternatively, use [`on_action_initiated_insert_from_resource`] to insert an instance from a [`Resource`].
pub fn on_action_initiated_insert_default<Action: Component + Default>(
    trigger: Trigger<OnActionInitiated, Action>,
    mut commands: Commands,
) {
    let actor = trigger.entity();
    commands.entity(actor).insert(Action::default());
}

/// [`Observer`] that listens for [`OnActionInitiated`] events targeting
/// the specified `Action` [`Component`] and inserts a [`Clone`] of the instance held in a [`Resource`]
/// onto the actor entity.
///
/// Alternatively, use [`on_action_initiated_insert_default`] to insert a [`Default`] instance.
pub fn on_action_initiated_insert_from_resource<Action: Component + Resource + Clone>(
    trigger: Trigger<OnActionInitiated, Action>,
    mut commands: Commands,
    resource: Res<Action>,
) {
    let actor = trigger.entity();
    commands.entity(actor).insert(resource.clone());
}

/// [`Observer`] that listens for [`OnActionEnded`] events targeting
/// the specified `Action` [`Component`] and removes the component from the actor entity.
pub fn on_action_ended_remove<Action: Component>(trigger: Trigger<OnActionEnded, Action>, mut commands: Commands) {
    let actor = trigger.entity();
    commands.entity(actor).remove::<Action>();
}
