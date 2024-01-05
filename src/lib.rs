use std::marker::PhantomData;

use bevy::{
    ecs::system::{EntityCommands, StaticSystemParam, SystemParam},
    prelude::*,
    reflect::GetTypeRegistration,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, SystemSet)]
pub struct BlueprintsSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, SystemSet)]
pub enum BlueprintSet {
    #[default]
    Cleanup,
    Sync,
    Flush,
}

pub trait FromBlueprint<T> {
    type Params<'w, 's>: SystemParam;

    fn from_blueprint(
        blueprint: &T,
        params: &mut StaticSystemParam<Self::Params<'_, '_>>,
    ) -> Self;
}

#[derive(Debug, Component, Default, Reflect)]
#[reflect(Component)]
pub struct Blueprint<B: Default>(B);

impl<B: Default> Blueprint<B> {
    pub fn new(data: B) -> Self {
        Blueprint(data)
    }
}

pub struct AsSelf;
pub struct AsChild;

pub trait BlueprintTarget {
    fn remove_target_bundle<T, P: Bundle + FromBlueprint<T>>(entity: &mut EntityCommands);

    fn attach_target_bundle<T, P: Bundle + FromBlueprint<T>>(
        entity: &mut EntityCommands,
        bundle: P,
    );

    // do nothing by default, but if AsChild, clean up orphaned children
    fn cleanup_despawned(
        _commands: &mut Commands,
        _entity: Entity,
        _query: &Query<(Entity, &Parent)>,
    ) {
    }
}

impl BlueprintTarget for AsSelf {
    fn remove_target_bundle<T, P: Bundle + FromBlueprint<T>>(entity: &mut EntityCommands) {
        entity.remove::<P>();
    }

    fn attach_target_bundle<T, P: Bundle + FromBlueprint<T>>(
        entity: &mut EntityCommands,
        bundle: P,
    ) {
        entity.insert(bundle);
    }
}

impl BlueprintTarget for AsChild {
    fn remove_target_bundle<T, P: Bundle + FromBlueprint<T>>(entity: &mut EntityCommands) {
        entity.despawn_descendants();
    }

    fn attach_target_bundle<T, P: Bundle + FromBlueprint<T>>(
        entity: &mut EntityCommands,
        bundle: P,
    ) {
        entity.with_children(|builder| {
            builder.spawn(bundle);
        });
    }

    // clean up orphaned children
    fn cleanup_despawned(
        commands: &mut Commands,
        parent_entity: Entity,
        query: &Query<(Entity, &Parent)>,
    ) {
        for (child_entity, parent) in query.iter() {
            if parent.get() == parent_entity {
                commands.entity(child_entity).despawn_recursive();
            }
        }
    }
}

pub struct BlueprintPlugin<B, P: Bundle + FromBlueprint<B>, T: BlueprintTarget = AsSelf> {
    blueprint_marker: PhantomData<B>,
    prefab_marker: PhantomData<P>,
    target_marker: PhantomData<T>,
}

impl<B, P, T> Default for BlueprintPlugin<B, P, T>
where
    P: Bundle + FromBlueprint<B>,
    T: BlueprintTarget,
{
    fn default() -> Self {
        Self {
            blueprint_marker: PhantomData::<B>,
            prefab_marker: PhantomData::<P>,
            target_marker: PhantomData::<T>,
        }
    }
}

impl<B, P, T> BlueprintPlugin<B, P, T>
where
    B: Default + Send + Sync + 'static,
    P: Bundle + FromBlueprint<B>,
    T: BlueprintTarget,
{
    fn sync_blueprint_prefab(
        mut commands: Commands,
        blueprint_query: Query<(Entity, &Blueprint<B>), Changed<Blueprint<B>>>,
        mut system_params: StaticSystemParam<P::Params<'_, '_>>,
    ) {
        for (entity, blueprint) in blueprint_query.iter() {
            let mut entity_commands = commands.entity(entity);
            T::remove_target_bundle::<B, P>(&mut entity_commands);
            T::attach_target_bundle::<B, P>(
                &mut entity_commands,
                P::from_blueprint(&blueprint.0, &mut system_params),
            );
        }
    }

    fn handle_removed_blueprints(
        mut commands: Commands,
        mut blueprint_query: RemovedComponents<Blueprint<B>>,
        child_query: Query<(Entity, &Parent)>,
    ) {
        for entity in blueprint_query.read() {
            if let Some(mut entity_commands) = commands.get_entity(entity) {
                T::remove_target_bundle::<B, P>(&mut entity_commands);
            } else {
                T::cleanup_despawned(&mut commands, entity, &child_query);
            }
        }
    }
}

impl<B, P, T> Plugin for BlueprintPlugin<B, P, T>
where
    B: Default + GetTypeRegistration + FromReflect + TypePath + Send + Sync + 'static,
    P: Bundle + FromBlueprint<B>,
    T: BlueprintTarget + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::handle_removed_blueprints.in_set(BlueprintSet::Cleanup),
                Self::sync_blueprint_prefab.in_set(BlueprintSet::Sync),
            ),
        );
        #[cfg(debug_assertions)]
        app.register_type::<Blueprint<B>>().register_type::<B>();
    }
}

pub struct BlueprintsPlugin;

impl Plugin for BlueprintsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                BlueprintSet::Cleanup,
                BlueprintSet::Sync,
                BlueprintSet::Flush,
            )
                .chain()
                .in_set(BlueprintsSet),
        )
        .add_systems(Update, apply_deferred.in_set(BlueprintSet::Flush));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[derive(Clone, Default, Reflect)]
    struct Rect {
        size: Vec2,
    }

    #[derive(Clone, Component, Default, Reflect)]
    #[reflect(Component)]
    struct RectSize(Vec2);

    #[derive(Clone, Component, Default, Reflect)]
    #[reflect(Component)]
    struct RectColor(Color);

    #[derive(Clone, Component, Default, Reflect)]
    #[reflect(Component)]
    struct RectArea(f32);

    #[test]
    fn single_blueprint() {
        #[derive(Bundle)]
        struct RectBundle {
            size: RectSize,
            color: RectColor,
            area: RectArea,
        }

        impl FromBlueprint<Rect> for RectBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                blueprint: &Rect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                RectBundle {
                    size: RectSize(blueprint.size),
                    color: RectColor(Color::RED),
                    area: RectArea(blueprint.size.x * blueprint.size.y),
                }
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BlueprintsPlugin))
            // whereas BlueprintsPlugin prepares the framework, individual BlueprintPlugins must
            // be added to define which blueprints to reactively manage
            .add_plugins(BlueprintPlugin::<Rect, RectBundle>::default());
        // when spawning an entity of this kind, spawn a Blueprint::<MyType>
        // (note: MyType does not have to be a component, but it can be)
        let entity = app.world.spawn(Blueprint::<Rect>::default()).id();
        app.update();
        assert_eq!(
            app.world
                .query::<(&RectSize, &RectColor, &RectArea)>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            1
        );
        assert_eq!(
            app.world
                .query::<Or<(&RectSize, &RectColor, &RectArea)>>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            1
        );
        // when the blueprint component is removed, the additional components are removed as well
        app.world.entity_mut(entity).remove::<Blueprint<Rect>>();
        app.update();
        assert_eq!(
            app.world
                .query::<Or<(&RectSize, &RectColor, &RectArea)>>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            0
        );
    }

    #[test]
    fn multiple_blueprint() {
        #[derive(Default, Reflect)]
        pub struct SpecificRect;

        #[derive(Bundle)]
        struct RectBundle {
            size: RectSize,
            color: RectColor,
            area: RectArea,
        }

        impl FromBlueprint<Rect> for RectBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                blueprint: &Rect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                RectBundle {
                    size: RectSize(blueprint.size),
                    color: RectColor(Color::RED),
                    area: RectArea(blueprint.size.x * blueprint.size.y),
                }
            }
        }

        impl FromBlueprint<SpecificRect> for RectBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                _: &SpecificRect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                let rect: Rect = Rect {
                    size: Vec2::new(10., 10.),
                };
                RectBundle {
                    size: RectSize(rect.size),
                    color: RectColor(Color::BLUE),
                    area: RectArea(rect.size.x * rect.size.y),
                }
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BlueprintsPlugin))
            // can add as many blueprints as we want (including overlapping ones, be careful!)
            .add_plugins(BlueprintPlugin::<Rect, RectBundle>::default())
            .add_plugins(BlueprintPlugin::<SpecificRect, RectBundle>::default());
        // spawn one entity for each blueprint
        let entity1 = app.world.spawn(Blueprint::<Rect>::default()).id();
        let entity2 = app.world.spawn(Blueprint::<SpecificRect>::default()).id();
        app.update();
        assert_eq!(
            app.world
                .query::<(&RectSize, &RectColor, &RectArea)>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            2
        );
        // clean up one at a time
        app.world.entity_mut(entity1).remove::<Blueprint<Rect>>();
        app.update();
        assert_eq!(
            app.world
                .query::<Or<(&RectSize, &RectColor, &RectArea)>>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            1
        );
        app.world
            .entity_mut(entity2)
            .remove::<Blueprint<SpecificRect>>();
        app.update();
        assert_eq!(
            app.world
                .query::<Or<(&RectSize, &RectColor, &RectArea)>>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            0
        );
    }

    #[test]
    fn multiple_prefab() {
        #[derive(Bundle)]
        struct RectBundle {
            size: RectSize,
            color: RectColor,
        }

        impl FromBlueprint<Rect> for RectBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                blueprint: &Rect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                RectBundle {
                    size: RectSize(blueprint.size),
                    color: RectColor(Color::RED),
                }
            }
        }

        // for some reason I need this bundle spawned separately
        // e.g. it is used in a separate crate or something
        #[derive(Bundle)]
        struct SecondRectBundle {
            area: RectArea,
        }

        impl FromBlueprint<Rect> for SecondRectBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                blueprint: &Rect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                SecondRectBundle {
                    area: RectArea(blueprint.size.x * blueprint.size.y),
                }
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BlueprintsPlugin))
            // can add multiple prefabs for the same blueprint component
            .add_plugins(BlueprintPlugin::<Rect, RectBundle>::default())
            .add_plugins(BlueprintPlugin::<Rect, SecondRectBundle>::default());
        // spawn the blueprint component
        let entity = app.world.spawn(Blueprint::<Rect>::default()).id();
        app.update();
        assert_eq!(
            app.world
                .query::<(&RectSize, &RectColor, &RectArea)>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            1
        );
        // cleanup is handled automatically
        app.world.entity_mut(entity).remove::<Blueprint<Rect>>();
        app.update();
        assert_eq!(
            app.world
                .query::<Or<(&RectSize, &RectColor, &RectArea)>>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            0
        );
    }

    #[test]
    fn self_and_child() {
        #[derive(Bundle)]
        struct RectBundle {
            color: RectColor,
        }

        impl FromBlueprint<Rect> for RectBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                _: &Rect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                RectBundle {
                    color: RectColor(Color::RED),
                }
            }
        }

        #[derive(Bundle)]
        struct RectChildBundle {
            area: RectArea,
            size: RectSize,
        }

        impl FromBlueprint<Rect> for RectChildBundle {
            type Params<'w, 's> = ();
            fn from_blueprint(
                blueprint: &Rect,
                _: &mut StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Self {
                RectChildBundle {
                    size: RectSize(blueprint.size),
                    area: RectArea(blueprint.size.x * blueprint.size.y),
                }
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BlueprintsPlugin))
            // can add multiple prefabs for the same blueprint component
            .add_plugins(BlueprintPlugin::<Rect, RectBundle>::default())
            .add_plugins(BlueprintPlugin::<Rect, RectChildBundle, AsChild>::default());
        // spawn the blueprint component
        let entity = app.world.spawn(Blueprint::<Rect>::default()).id();
        app.update();
        let parent_entities = app
            .world
            .query::<(Entity, &RectColor, &Children)>()
            .iter(&app.world)
            .collect::<Vec<_>>();
        assert_eq!(parent_entities.len(), 1);
        let parent_entity = parent_entities.first().unwrap().0;
        let child_entities = app
            .world
            .query::<(&RectSize, &RectArea, &Parent)>()
            .iter(&app.world)
            .collect::<Vec<_>>();
        assert_eq!(child_entities.len(), 1);
        assert_eq!(child_entities.first().unwrap().2.get(), parent_entity);
        // cleanup is handled automatically
        app.world.entity_mut(entity).remove::<Blueprint<Rect>>();
        app.update();
        assert_eq!(
            app.world
                .query::<Or<(&RectSize, &RectColor, &RectArea)>>()
                .iter(&app.world)
                .collect::<Vec<_>>()
                .len(),
            0
        );
    }
}
