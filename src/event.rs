//! Events that define the lifecycle of the library.
//!
//! These events are split into three categories:
//! - Scoring events
//! - Picking events
//! - Acting events
//!
//! Generally speaking, events that start with `On` should only be listened to,
//! while other events should only be triggered.
//!
//! # Scoring events
//!
//! [`RunScoring`] can be triggered to score a specific entity or all entities with the [`Score`] component.
//! [`Score`] entities with [`Score`] children will be scored after their children, to ensure correct scoring.
//! This will trigger the [`OnScore`] event for the target entity, which should be listened to by scoring [`Observer`]s
//! to calculate the [`Score`] for a given entity.
//!
//! # Picking events
//!
//! [`RunPicking`] can be triggered to make a specific entity or all entities with the [`Picker`] component pick an action.
//! This will trigger the [`OnPick`] event for the target entity, which should be listened to by picking [`Observer`]s and
//! which will trigger the [`OnPicked`] event with the picked action.
//!
//! # Acting events
//!
//! [`RequestAction`] can be triggered to request an action to be initiated for a specific entity.
//! This will trigger the [`OnActionInitiated`] event for the target entity, using the action picked by their [`Picker`].
//! The [`OnActionEnded`] event is triggered by action lifecycle or actions themselves to indicate that they have completed or been cancelled.
//! In between these two previous events, the action should be executed.
//!
//! [`Score`]: crate::scoring::Score
//! [`Picker`]: crate::picking::Picker

use bevy::{ecs::component::ComponentId, prelude::*};

////////////////////////////////////////////////////////////
// Scoring events
////////////////////////////////////////////////////////////

/// Trigger this [`Event`] to score the targeted entity,
/// or all entities if no target is specified.
///
/// Entities are scored in depth-first post-order traversal,
/// ensuring that all children are scored before their parents.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct RunScoring;

/// This [`Event`] is listened to by scoring systems to calculate the score(s) for a given entity.
/// DO NOT TRIGGER MANUALLY, trigger [`RunScoring`] instead.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct OnScore;

////////////////////////////////////////////////////////////
// Picking events
////////////////////////////////////////////////////////////

/// Trigger this [`Event`] to make the target actor entity pick an action based on its score(s),
/// or all actor entities if no target is specified.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct RunPicking;

/// Listen to this [`Event`] to handle picking an action for the target actor entity.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct OnPick;

/// Listen to this [`Event`] to check which action was picked for the target actor entity.
/// This [`Event`] is triggered by [`Picker`]s to indicate that an action has been picked for the target actor entity.
///
/// [`Picker`]: crate::picking::Picker
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[reflect(Component, PartialEq, Debug)]
pub struct OnPicked {
    /// [`ComponentId`] of the action that was picked.
    pub action: ComponentId,
}

////////////////////////////////////////////////////////////
// Action events
////////////////////////////////////////////////////////////

/// Trigger this [`Event`] to request a specific action or the picked action to be initiated for the target actor entity.
///
/// This event SHOULD NOT be triggered without a target entity.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct RequestAction {
    /// The [`ComponentId`] of the action that was requested, if any.
    pub action: Option<ComponentId>,
}

/// This [`Event`] is triggered by action lifecycle to indicate that they have been initiated.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[reflect(Component, PartialEq, Debug)]
pub struct OnActionInitiated {
    /// [`ComponentId`] of the action that was initiated.
    pub action: ComponentId,
}

/// This [`Event`] is triggered by action lifecycle or actions themselves to indicate
/// that they have completed or been cancelled.
///
/// An action will be cancelled if a different action is [requested][`RequestAction`] before it completes.
#[derive(Event, Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[reflect(Component, PartialEq, Debug)]
pub struct OnActionEnded {
    /// [`ComponentId`] of the action that was finished.
    pub action: ComponentId,
    /// The reason the action was finished.
    pub reason: ActionEndReason,
}

impl OnActionEnded {
    /// Creates a new [`Completed`][`ActionEndReason::Completed`] [`OnActionEnded`] event with the given action.
    #[must_use]
    pub fn completed(action: ComponentId) -> Self {
        Self {
            action,
            reason: ActionEndReason::Completed,
        }
    }

    /// Creates a new [`Cancelled`][`ActionEndReason::Cancelled`] [`OnActionEnded`] event with the given action.
    #[must_use]
    pub fn cancelled(action: ComponentId) -> Self {
        Self {
            action,
            reason: ActionEndReason::Cancelled,
        }
    }
}

/// The reason [`OnActionEnded`] was triggered.
#[derive(Reflect)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[reflect(PartialEq, Debug)]
pub enum ActionEndReason {
    /// The action was completed successfully.
    Completed,
    /// The action was cancelled.
    Cancelled,
}
