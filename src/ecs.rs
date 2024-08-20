//! [`bevy`] ECS utilities for implementing library functionality.

use std::{collections::VecDeque, iter::FusedIterator, marker::PhantomData};

use bevy::{
    ecs::{
        component::ComponentId,
        entity::EntityHashMap,
        observer::TriggerTargets,
        query::{QueryData, QueryEntityError, QueryFilter, ReadOnlyQueryData},
        system::{IntoObserverSystem, SystemParam},
        world::Command,
    },
    prelude::*,
    utils::Entry,
};

/// A [`TriggerTargets`] used by the action [`Event`]s to trigger an action [`ComponentId`] for a given entity.
pub struct TargetedAction(pub Entity, pub ComponentId);

impl TriggerTargets for TargetedAction {
    #[inline]
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        std::iter::once(self.1)
    }

    #[inline]
    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        std::iter::once(self.0)
    }
}

/// [`Trigger`] extension trait that provides checked access to [`Trigger::entity`].
pub trait TriggerGetEntity {
    /// Returns the triggered [`Entity`], but only if it's not [`Entity::PLACEHOLDER`].
    fn get_entity(&self) -> Option<Entity>;
}

impl<E, B: Bundle> TriggerGetEntity for Trigger<'_, E, B> {
    #[inline]
    fn get_entity(&self) -> Option<Entity> {
        Some(self.entity()).filter(|e| e != &Entity::PLACEHOLDER)
    }
}

/// A [`Query`] wrapper that finds the closest ancestor entity with a given component.
/// Uses a cache to speed up subsequent queries.
#[derive(SystemParam)]
pub struct AncestorQuery<'w, 's, T: ReferenceType> {
    /// The query to find the component, crawling up the hierarchy if necessary.
    check: Query<'w, 's, (<T as ReferenceType>::Has, Option<&'static Parent>)>,
    /// The query to grab the component. This query wouldn't be necessary if rust wouldn't complain!
    fetch: Query<'w, 's, T>,
    /// Caches a given entity's closest ancestor entity with the component T.
    cache: Local<'s, EntityHashMap<Entity>>,
}

impl<'w, 's, T: ReferenceType> AncestorQuery<'w, 's, T> {
    /// Crawls up the hierarchy to find the closest ancestor entity with the component `T`.
    fn find(&mut self, start: Entity) -> Result<Entity, QueryEntityError> {
        // Crawl up the hierarchy
        let mut current = start;
        loop {
            match self.check.get(current) {
                Ok((true, _)) => {
                    // Found the component, cache it and return
                    self.cache.insert(start, current);
                    return Ok(current);
                }
                Ok((false, Some(parent))) => {
                    // Continue searching up the hierarchy
                    current = **parent;
                }
                Ok((false, None)) | Err(_) => {
                    // No parent with the component found
                    return Err(QueryEntityError::NoSuchEntity(current));
                }
            }
        }
    }

    /// Clears the cache to free up memory, if necessary.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl<'w, 's, T: Component> AncestorQuery<'w, 's, &'static T> {
    /// Returns a readonly reference to the [`Component`] `T` on the closest ancestor entity, if any.
    ///
    /// # Errors
    ///
    /// If the entity does not exist or the component is not found.
    pub fn get(&mut self, start: Entity) -> Result<&T, QueryEntityError> {
        // Check the cache first
        if let Entry::Occupied(entry) = self.cache.entry(start) {
            if self.fetch.contains(*entry.get()) {
                // Cache hit
                return self.fetch.get(*entry.get());
            }

            // Cache miss
            entry.remove();
        }

        self.find(start).and_then(|found| self.fetch.get(found))
    }
}

impl<'w, 's, T: Component> AncestorQuery<'w, 's, &'static mut T> {
    /// Returns a mutable reference to the [`Component`] `T` on the closest ancestor entity, if any.
    ///
    /// # Errors
    ///
    /// If the entity does not exist or the component is not found.
    pub fn get_mut(&mut self, start: Entity) -> Result<Mut<T>, QueryEntityError> {
        // Check the cache first
        if let Entry::Occupied(entry) = self.cache.entry(start) {
            if self.fetch.contains(*entry.get()) {
                // Cache hit
                return self.fetch.get_mut(*entry.get());
            }

            // Cache miss
            entry.remove();
        }

        self.find(start).and_then(|found| self.fetch.get_mut(found))
    }
}

/// A [`QueryData`] supertrait for `&T` and `&mut T` reference types.
pub trait ReferenceType: QueryData + 'static {
    /// The [`Has`] type for this reference type.
    type Has: for<'a> ReadOnlyQueryData<Item<'a> = bool>;
}

impl<T: Component> ReferenceType for &'static T {
    type Has = Has<T>;
}

impl<T: Component> ReferenceType for &'static mut T {
    type Has = Has<T>;
}

/// [`Command`] that runs a given command only if the [`Resource`] `R` has not been inserted into the [`World`] yet.
/// After running the command, the resource is inserted into the world.
pub struct Once<R: Resource + Default, C: Command> {
    _type: PhantomData<R>,
    command: C,
}

impl<R: Resource + Default, C: Command> Command for Once<R, C> {
    fn apply(self, world: &mut World) {
        if world.contains_resource::<R>() {
            // We've already run the command.
            return;
        }
        world.insert_resource(R::default());
        self.command.apply(world);
    }
}

/// A [`Commands`] wrapper that provides a way to run commands only, based on the presence of [`Resource`] `R`.
///
/// See [`CommandsExt::once`] for more information.
pub struct OnceCommands<'w, 's, R: Resource + Default> {
    commands: Commands<'w, 's>,
    _type: PhantomData<R>,
}

impl<'w, 's, R: Resource + Default> OnceCommands<'w, 's, R> {
    fn new(commands: Commands<'w, 's>) -> Self {
        Self {
            commands,
            _type: PhantomData,
        }
    }

    /// Adds the specified [`Observer`] system if and only if the [`Resource`] `R` has not been inserted into the [`World`] yet.
    /// After running the command, the resource is inserted into the world.
    pub fn observe<E: Event, B: Bundle, M>(mut self, observer: impl IntoObserverSystem<E, B, M>) {
        self.commands.add(Once::<R, _> {
            _type: PhantomData,
            command: |world: &mut World| {
                world.observe(observer);
            },
        });
    }
}

/// [`Commands`] extension trait for library-specific commands.
pub trait CommandsExt {
    /// Returns a [`Commands`] wrapper that provides a way to run commands only once, based on the presence of [`Resource`] `R`.
    fn once<R: Resource + Default>(&mut self) -> OnceCommands<'_, '_, R>;
}

impl CommandsExt for Commands<'_, '_> {
    fn once<R: Resource + Default>(&mut self) -> OnceCommands<'_, '_, R> {
        OnceCommands::new(self.reborrow())
    }
}

/// [`SystemParam`] that provides a depth-first search post-order traversal of the entity hierarchy,
/// starting from a given root [`Entity`].
#[derive(SystemParam)]
pub struct DFSPostTraversal<'w, 's, F: QueryFilter + 'static = ()> {
    children: Query<'w, 's, &'static Children, F>,
    queue: Local<'s, VecDeque<(usize, Entity)>>,
}

impl<'w, 's, F: QueryFilter + 'static> DFSPostTraversal<'w, 's, F> {
    /// Returns an iterator that provides a depth-first search post-order traversal of the entity hierarchy,
    /// starting from a given root [`Entity`].
    ///
    /// The deepest children are visited first, followed by their parents.
    pub fn iter(&mut self, root: Entity) -> DFSPostTraversalIter<'_, 'w, 's, F> {
        DFSPostTraversalIter::new(self, root)
    }
}

/// [`Iterator`] type returned by [`DFSPostTraversal::iter`].
pub struct DFSPostTraversalIter<'a, 'w, 's, F: QueryFilter + 'static> {
    param: &'a mut DFSPostTraversal<'w, 's, F>,
    visited: usize,
    current_depth: usize,
}

impl<'a, 'w, 's, F: QueryFilter + 'static> DFSPostTraversalIter<'a, 'w, 's, F> {
    fn new(param: &'a mut DFSPostTraversal<'w, 's, F>, root: Entity) -> Self {
        param.queue.clear();
        param.queue.push_back((0, root));

        Self {
            param,
            visited: 0,
            current_depth: 0,
        }
    }
}

impl<F: QueryFilter + 'static> Iterator for DFSPostTraversalIter<'_, '_, '_, F> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        if self.param.queue.is_empty() {
            return None;
        }

        // Exhaust all children for the first branch
        loop {
            let i = self.visited;
            let Some(&(depth, entity)) = self.param.queue.get(i) else {
                break;
            };

            // This node is not a child nor a sibling
            if self.current_depth > depth {
                break;
            }

            self.visited += 1;
            self.current_depth = depth;

            let Ok(entity_children) = self.param.children.get(entity) else {
                // No children
                break;
            };

            // TODO: can we replace this with some kind of `extend_at`?
            for (j, child) in entity_children.into_iter().copied().enumerate() {
                self.param.queue.insert(i + j + 1, (depth + 1, child));
            }
        }

        let (depth, entity) = self.param.queue.remove(self.visited - 1)?;

        self.visited -= 1;
        self.current_depth = depth;

        Some(entity)
    }
}

impl<F: QueryFilter + 'static> FusedIterator for DFSPostTraversalIter<'_, '_, '_, F> {}
