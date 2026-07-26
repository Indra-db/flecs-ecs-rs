use super::*;

/// Observer mixin implementation
impl World {
    /// Upcast entity to an observer.
    /// The provided entity must be an observer.
    ///
    /// # Arguments
    ///
    /// * `e` - The entity.
    ///
    /// # Returns
    ///
    /// An observer object.
    pub fn observer_from<'a>(&'a self, e: EntityView<'a>) -> Observer<'a> {
        Observer::new_from_existing(e)
    }

    /// Create a new observer.
    ///
    /// # Type Parameters
    ///
    /// * `Components` - The components to match on.
    ///
    /// # Returns
    ///
    /// Observer builder.
    ///
    /// # See also
    ///
    /// * [`World::observer_from()`]
    /// * [`World::observer_id()`]
    /// * [`World::observer_named()`]
    pub fn observer<Event: ComponentId, Components>(&self) -> ObserverBuilder<'_, Event, Components>
    where
        Components: QueryTuple,
    {
        ObserverBuilder::<Event, Components>::new(self)
    }

    pub fn observer_id<Components>(
        &self,
        event: impl Into<Entity>,
    ) -> ObserverBuilder<'_, (), Components>
    where
        Components: QueryTuple,
    {
        let builder = ObserverBuilder::<(), Components>::new_untyped(self);
        builder.add_event(event)
    }

    /// Create a new named observer.
    ///
    /// # Type Parameters
    ///
    /// * `Components` - The components to match on.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the observer.
    ///
    /// # Returns
    ///
    /// Observer builder.
    ///
    /// # See also
    ///
    /// * [`World::observer_from()`]
    /// * [`World::observer()`]
    /// * [`World::observer_id()`]
    pub fn observer_named<'a, Event: ComponentId, Components>(
        &'a self,
        name: &str,
    ) -> ObserverBuilder<'a, Event, Components>
    where
        Components: QueryTuple,
    {
        ObserverBuilder::<Event, Components>::new_named(self, name)
    }
}
