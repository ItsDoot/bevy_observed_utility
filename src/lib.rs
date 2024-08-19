//! A state-of-the-art [utility AI](https://en.wikipedia.org/wiki/Utility_system) library for [Bevy Engine](https://bevyengine.org/).
//!
//! # Design Goals
//!
//! In order of priority:
//!
//! - **Correctness**
//!   - Scoring entity trees are scored in depth-first post-order traversal, ensuring that all children are scored before their parents.
//! - **Ergonomics**:
//!   - Adding scoring, picking, and actions to entities should have little boilerplate.
//! - **Modularity**:
//!   - Adding new kinds of scoring and picking should be easy.
//!   - Adding different ways of handling actions should be easy.
//!   - Both turn-based and real-time games should be supported.
//! - **Performance**:
//!   - Pay only for what you use: Scoring and picking observers are only added if they are used.
//!   - Scoring and picking should be reasonably fast. Action performance is up to the user.
//!
//! # Crate Layout
//!
//! Utility AI is typically implemented as a 3-part lifecycle, and this library follows that pattern:
//! - [**Scoring**](crate::scoring): Entities evaluate the world around them and assign scores to themselves based on that evaluation.
//! - [**Picking**](crate::acting): Entities choose actions based on their scores.
//! - [**Acting**](crate::acting): Entities perform actions based on their picks.
//!
//! ## Scoring
//!
//! In this crate, actor entities hold the view into the world,
//! and child [`Score`] entities hold and calculate the scores based on that view.
//! These children can be nested to any depth, giving you the flexibility to score entities however complex you want.
//!
//! # Full Walkthrough Example
//!
//! ```rust
//! use bevy::{prelude::*, ecs::component::ComponentId};
//! use bevy_observed_utility::prelude::*;
//!
//! // To start our game, we'll need to create a new App.
//! let mut app = App::new();
//! // We'll also need to add the ObservedUtilityPlugins group which handles the whole lifecycle for us,
//! // leaving us to handle the features.
//! app.add_plugins(ObservedUtilityPlugins::RealTime);
//!
//! /// Define a component which provides a view into the world.
//! /// Just a normal component for our actor's thirst.
//! #[derive(Component)]
//! struct Thirst {
//!     /// Goes from 0 to 100.
//!     value: f32,
//!     /// How much thirst increases per second.
//!     per_second: f32,
//! }
//!
//! /// This impl allows us to use the score_ancestor function to score thirst, later on.
//! impl From<&Thirst> for Score {
//!    fn from(thirst: &Thirst) -> Self {
//!       Score::new(thirst.value / 100.)
//!    }
//! }
//!
//! /// Update the actor's view into the world however you like. In this case, it's just slowly increasing over time.
//! /// Still just a normal system that runs at a fixed rate.
//! fn get_thirsty_over_time(time: Res<Time<Fixed>>, mut actors: Query<&mut Thirst>) {
//!    for mut thirst in actors.iter_mut() {
//!        thirst.value = (thirst.value + thirst.per_second * time.delta_seconds()).min(100.);
//!    }
//! }
//! app.add_systems(FixedUpdate, get_thirsty_over_time);
//!
//! /// Now lets define a component to mark the score entity as measuring thirst.
//! /// As opposed to Thirst, this one stores no data; it's just a marker.
//! #[derive(Component)]
//! pub struct Thirsty;
//!
//! // The first of a few library-provided functions to reduce boilerplate is score_ancestor.
//! // It listens for the OnScore event and scores the entity using two generic parameters:
//! // - #1: The component to read from the closest parent/ancestor entity.
//! // - #2: The score entity with this marker component.
//! app.observe(score_ancestor::<Thirst, Thirsty>);
//!
//! /// Next we need an action to perform when the thirst is high enough.
//! /// This one also belongs to the actor, and is just a normal component.
//! #[derive(Component)]
//! pub struct Drinking {
//!     /// This one's also from 0 to 100.
//!     pub until: f32,
//!     /// How much thirst is quenched per second.
//!     pub per_second: f32,
//! }
//!
//! /// This impl is used when inserting the component onto the actor.
//! /// (its required for the on_action_initiated_insert_default observer later on)
//! impl Default for Drinking {
//!     fn default() -> Self {
//!         Self {
//!             until: 10.,
//!             per_second: 2.,
//!         }
//!     }
//! }
//!
//! /// We'll also need a default action for the actor to perform when it's not doing anything else.
//! #[derive(Component)]
//! pub struct Idle;
//!
//! /// We'll also need ComponentIds for these actions to later identify and perform lifecycle events on them.
//! #[derive(Resource)]
//! pub struct ActionIds {
//!     pub idle: ComponentId,
//!     pub drinking: ComponentId,
//! }
//!
//! /// We'll need to initialize the action ids somewhere later on.
//! /// Unfortunately initializing ComponentIds requires mutable access to the world, so we'll use FromWorld.
//! impl FromWorld for ActionIds {
//!     fn from_world(world: &mut World) -> Self {
//!         Self {
//!             idle: world.init_component::<Idle>(),
//!             drinking: world.init_component::<Drinking>(),
//!         }
//!     }
//! }
//!
//! // Go ahead and initialize it on the App:
//! app.init_resource::<ActionIds>();
//!
//! // The library provides a builtin function to handle the common case where
//! // an action is initiated and its component should be inserted onto the actor.
//! // Inserting this component will allow the actor to be targeted in the following system.
//! app.observe(on_action_initiated_insert_default::<Drinking>);
//!
//! /// Now we'll update all actors that are drinking.
//! /// This is a normal system that runs at a fixed rate.
//! fn drink(
//!     mut commands: Commands,
//!     time: Res<Time<Fixed>>,
//!     mut actors: Query<(Entity, &mut Thirst, &Drinking)>,
//!     actions: Res<ActionIds>
//! ) {
//!     for (actor, mut thirst, drinking) in actors.iter_mut() {
//!         // Quench the thirst a bit.
//!         thirst.value = (thirst.value - drinking.per_second * time.delta_seconds()).max(0.);
//!         // If the thirst is low enough, finish drinking.
//!         if thirst.value <= drinking.until {
//!             /// We'll need that ActionIds resource we created earlier to identify the action.
//!             commands.trigger_targets(
//!                 OnActionEnded::completed(actions.drinking),
//!                 TargetedAction(actor, actions.drinking),
//!             );
//!         }
//!     }
//! }
//! // Lets add the system to the app, and order it after the get_thirsty_over_time system.
//! app.add_systems(FixedUpdate, drink.after(get_thirsty_over_time));
//!
//! // Similar to on_action_initiated_insert_default, the library also provides a function
//! // to automatically remove the action's component from the actor when it ends.
//! app.observe(on_action_ended_remove::<Drinking>);
//!
//! // Now onto the real magic: spawning our entities!
//!
//! // We'll need a system that's executed on startup to spawn our actor entity and child score entity.
//! fn spawn_entities(mut commands: Commands, actions: Res<ActionIds>) {
//!     // Let's build the tree from the bottom up, since it'll be easier to insert the Picker on the actor last.
//!     // First, the entity that scores thirst.
//!     let thirst = commands.spawn((Thirsty, Score::default())).id();
//!     
//!     // Spawn the actor entity
//!     commands
//!         .spawn((
//!             // Remember the first component we defined? Put it on the actor.
//!             Thirst {
//!                 value: 0.,
//!                 per_second: 1.,
//!             },
//!             // We actually have one more concept to introduce: the Picker.
//!             // The Picker is a component that tells the system which action to perform based on the scores.
//!             // All pickers need a default action to perform when they're not doing anything else.
//!             Picker::new(actions.idle)
//!                 // When the actor gets thirsty enough, they'll drink.
//!                 .with(thirst, actions.drinking),
//!             // To configure the picker's selection behavior, we insert a component that handles that.
//!             // In this case, we'll insert the FirstToScore component,
//!             // which picks the first action that scores above a certain threshold.
//!             // There's other picker variants too, like Random and Highest.
//!             FirstToScore::new(0.5),
//!             // For action handling we'll need one final component: CurrentAction.
//!             // This component holds the ComponentId of the action the actor is currently performing.
//!             // Which makes it easy to check what the actor is doing.
//!             // We'll spawn the actor idling.
//!             CurrentAction(actions.idle),
//!         ))
//!         .add_child(thirst);
//! }
//!
//! // Register the system and run the app!
//! app.add_systems(Startup, spawn_entities);
//! app.run();
//! // Done!
//! ```
//!
//! # Optimizing for Performance
//!
//! While this library does its best to be performant, there are a few ways to improve performance in your game:
//! - Run scoring and picking systems at a slower fixed rate than default.
//!     - This will induce latency in the AI, but will reduce overall frame time.
//! - Replace deeply nested scoring hierarchies with shallow hand-written scoring observers.
//!
//! [`Score`]: crate::scoring::Score

#![warn(missing_docs)]

use bevy::{
    app::PluginGroupBuilder,
    ecs::schedule::{InternedScheduleLabel, ScheduleLabel},
    prelude::*,
};

use crate::{
    acting::{ActionPlugin, CurrentAction},
    event::{RequestAction, RunPicking, RunScoring},
    picking::{Picker, PickingPlugin},
    scoring::ScoringPlugin,
};

pub mod acting;
pub mod ecs;
pub mod event;
pub mod picking;
pub mod scoring;

pub mod prelude {
    //! Re-exports important traits and types.
    pub use crate::{
        acting::{
            on_action_ended_remove, on_action_initiated_insert_default, on_action_initiated_insert_from_resource,
            CurrentAction,
        },
        ecs::{AncestorQuery, TargetedAction},
        event::{ActionEndReason, OnActionEnded, OnActionInitiated, OnPick, OnPicked, OnScore, RunPicking, RunScoring},
        picking::{FirstToScore, Highest, Picker},
        scoring::{
            score_ancestor, AllOrNothing, Evaluated, Evaluator, FixedScore, LinearEvaluator, Measure, Measured,
            PowerEvaluator, Product, Score, SigmoidEvaluator, Sum, Weighted, WeightedMax, WeightedProduct, WeightedRMS,
            WeightedSum, Winning,
        },
        ObservedUtilityPlugins,
    };

    #[cfg(feature = "rand")]
    pub use crate::{picking::PickRandom, scoring::RandomScore};
}

/// [`PluginGroup`] for all standard plugins in `bevy_observed_utility`.
pub enum ObservedUtilityPlugins {
    /// Config meant for real-time games.
    ///
    /// This includes the [`RealtimeLifecyclePlugin`] which automatically runs scoring, picking,
    /// and perf systems in the configured [`Schedule`] (default [`FixedPostUpdate`]).
    RealTime,
    /// Config meant for turn-based games.
    ///
    /// This does NOT include the [`RealtimeLifecyclePlugin`],
    /// so you'll need to run scoring, picking, and action selection manually.
    ///
    /// To do so, trigger the [`RunScoring`] and [`RunPicking`] events un-targeted,
    /// which will score and pick actions for all entities with the appropriate components.
    /// Then trigger the [`RequestAction`] event targeted at an actor entity when you want them to perform an action.
    TurnBased,
}

impl PluginGroup for ObservedUtilityPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(ScoringPlugin)
            .add(PickingPlugin)
            .add(ActionPlugin);
        match self {
            ObservedUtilityPlugins::RealTime => builder.add(RealtimeLifecyclePlugin::default()),
            ObservedUtilityPlugins::TurnBased => builder,
        }
    }
}

/// [`Plugin`] which automatically runs scoring, picking, and perf systems in the configured [`Schedule`].
/// This plugin is included in [`ObservedUtilityPlugins::RealTime`].
///
/// This plugin is meant for real-time games, but might be useful for turn-based games as well.
pub struct RealtimeLifecyclePlugin {
    /// The [`ScheduleLabel`] to run scoring and picking, and action selection in.                                                      
    pub score_pick_perform_in: InternedScheduleLabel,
}

impl Plugin for RealtimeLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            self.score_pick_perform_in,
            (Self::score_and_pick, Self::request_action_if_none_or_default),
        );
    }
}

impl Default for RealtimeLifecyclePlugin {
    fn default() -> Self {
        Self {
            score_pick_perform_in: FixedPostUpdate.intern(),
        }
    }
}

impl RealtimeLifecyclePlugin {
    /// [`System`] that automatically runs scoring and picking [`Observer`]s.
    pub fn score_and_pick(mut commands: Commands) {
        commands.trigger(RunScoring);
        commands.trigger(RunPicking);
    }

    /// [`System`] that requests a new action for an actor if they're currently "idling",
    /// i.e. performing their default action.
    pub fn request_action_if_none_or_default(
        mut commands: Commands,
        actors: Query<(Entity, &Picker, Option<&CurrentAction>)>,
    ) {
        for (actor, picker, current_action) in actors.iter() {
            if current_action.is_some_and(|ca| picker.is_default(ca.0)) || current_action.is_none() {
                commands.trigger_targets(RequestAction { action: None }, actor);
            }
        }
    }
}
