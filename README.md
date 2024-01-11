# Bevy Reactive Blueprints

This crate canonicalizes an approach to "blueprint"-based component patterns in Bevy.

This allows users to attach one component, referred to here as the _blueprint_, and automatically attaches an associated `Bundle` to the entity (or spawns the Bundle as a child entity). This crate refers to the associated `Bundle` and any child entities created by this process as _prefabs_ associated with the blueprint. (I do not know whether this is entirely appropriate, but the value provided by the crate is similar to prefabs in typical game development environments.)

The supported workflow involves defining a type that serves as a builder for some `Bundle`, attaching the appropriate plugin, and spawning entities with (or attaching to existing entities) a `Blueprint::new(my_type)`. The plugins provided in `bevy_reactive_blueprints` attach systems that reactively update and remove prefab data, so updates to a `Blueprint` will remove and re-attach components and children as necessary.

It is worth noting that the intention is _not_ to encourage frequently editing blueprint types in the application. Blueprints are best used for spawning and saving scenes or for development contexts.

In particular, this crate is used well alongside [`bevy_editor_pls`](https://github.com/jakobhellermann/bevy_editor_pls) so that the various "object kinds" in a given application can be spawned and manipulated using the blueprints. An example is provided in the `examples` directory. Scene files can also strictly save Blueprint components, minimizing the amount of data stored. ([`iyes_scene_tools`](https://github.com/IyesGames/iyes_scene_tools) is useful here, but an example is not provided yet.)

## Usage

### Blueprint/Prefab Types

First, define the types that will serve as your blueprints and prefab bundles. The blueprint type should implement `Default` and `bevy::prelude::Reflect`.

### Plugins

Add `BlueprintsPlugin` to your app:

```rust
let mut app = App::new();
// ...
app.add_plugins(BlueprintsPlugin);
```

This configures `BlueprintsSet`, a `SystemSet` where inner systems are attached, and adds `apply_deferred` so that commands are flushed after building the associated prefabs.

Then, individual blueprints can be defined by attaching a `BlueprintPlugin` for each pair of types that needs to be managed. `BlueprintPlugin` accepts three type parameters:

1. The type that will serve as the blueprint (which does not need to be a component but can be). This should implement `Default` and `Reflect`.
2. The `Bundle` that should be spawned, which needs to implement the `FromBlueprint<MyType>` trait ([see below](#fromblueprint)).
3. Optionally, the `AsChild` type can be used here to instantiate the prefab as a child entity.

```rust
let mut app = App::new();
// ...
app.add_plugins(BlueprintsPlugin);
app.add_plugins(BlueprintPlugin::<MyBlueprint, MyPrefabBundle>::default());
// OR
app.add_plugins(BlueprintPlugin::<MyBlueprint, MyPrefabBundle, AsChild>::default());
```

Blueprints can have various prefabs, leading to composable behavior:

```rust
let mut app = App::new();
// ...
// SelfPrefabBundle1 and SelfPrefabBundle2 will be attached to the same entity,
// and ChildPrefabBundle will spawn a child entity with the bundle.
app.add_plugins(BlueprintPlugin::<MyBlueprint, SelfPrefabBundle1>::default());
app.add_plugins(BlueprintPlugin::<MyBlueprint, SelfPrefabBundle2>::default());
app.add_plugins(BlueprintPlugin::<MyBlueprint, ChildPrefabBundle, AsChild>::default());
```

When doing this, be sure to respect Bevy's typical rules: if `SelfPrefabBundle1` and `SelfPrefabBundle2` share components, this will cause panics.

### FromBlueprint

In order for this to work, prefab bundles must implement the `FromBlueprint` trait. This requires defining the associated type `Params: SystemParam` which is used in the `from_blueprint` method to provide any system parameters necessary to perform the conversion.

If no system params are necessary, this might look as follows:

```rust
impl FromBlueprint<MyType> for MyPrefabBundle {
    // this is some bevy::ecs::SystemParam
    type Params<'w, 's> = ();

    fn from_blueprint(
        blueprint: &Rect,
        params: &mut StaticSystemParam<Self::Params<'_, '_>>,
    ) -> Self {
        MyPrefabBundle { /* ... */ }
    }
}
```

However, system parameters are frequently necessary to build prefab bundles, especially in order to access assets. This might look like the following:

```rust
#[derive(Bundle)]
struct MyPrefabBundle {
    pbr: PbrBundle,
}

#[derive(SystemParam)]
struct MyBlueprintParams<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl FromBlueprint<MyType> for MyPrefabBundle {
    type Params<'w, 's> = MyBlueprintParams<'w>;

    fn from_blueprint<'w, 's>(
        blueprint: &MyType,
        params: &mut StaticSystemParam<Self::Params<'w, 's>>,
    ) -> Self {
        MyPrefabBundle {
            pbr: PbrBundle {
                mesh: params.meshes.add(todo!()),
                material: params.materials.add(todo!()),
                transform: todo!(),
                ..default()
            },
        }
    }
}
```

See the tests (and the example in the editor crate) for more information.

## TODOs

- Add docstrings.
- Only require Reflect conditionally.
- Build a crate that defines a helpful window for `bevy_editor_pls`.
- Consider reorganizing so that an app extension trait could be used to register blueprint types. This might be more complicated than necessary, but would be nice for ergonomics.

## Caution

Beware that this calls `despawn_recursive` and `despawn_descendants` to handle cleanup, so attaching child entities that aren't related to any blueprint is probably going to cause problems.

If you have trouble getting the plugin to work, make sure that (1) your blueprint implements `Default` and `bevy::prelude::Reflect` and (2) your prefab implements `FromBlueprint`.
