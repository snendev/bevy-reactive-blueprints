# Bevy Reactive Blueprints

This crate canonicalizes an approach to "blueprint"-based component patterns in Bevy.

This allows users to attach one component, referred to here as the _blueprint_, and automatically attaches an associated `Bundle` to the entity (or spawns the Bundle as a child entity). This crate refers to the associated `Bundle` and any child entities created by this process as _prefabs_ associated with the blueprint. (I do not know whether this is entirely appropriate, but the value provided by the crate is similar to prefabs in typical game development environments.)

The supported workflow involves defining a type that serves as a builder for some `Bundle`, attaching the appropriate plugin, and spawning entities with (or attach to existing entities some) `Blueprint::new(my_type)`. The plugins provided in `bevy_reactive_blueprints` attach systems that reactively update and remove prefab data, so updates to a `Blueprint` will remove and re-attach components and children as necessary.

It is worth noting that the intention is _not_ to encourage frequently editing blueprint types in the application. Blueprints are best used for spawning and saving scenes or for development contexts.

In particular, this crate is used well alongside [`bevy_editor_pls`](https://github.com/jakobhellermann/bevy_editor_pls) so that the various "object kinds" in a given application can be spawned and manipulated using the blueprints. An example is provided in the `examples` directory. Scene files can also strictly save Blueprint components, minimizing the amount of data stored. ([`iyes_scene_tools`](https://github.com/IyesGames/iyes_scene_tools) is useful here, but an example is not provided yet.)

## Usage

First, add `BlueprintsPlugin` to your app:

```rust
let mut app = App::new();
// ...
app.add_plugins(BlueprintsPlugin);
```

This configures `BlueprintsSet`, a `SystemSet` where inner systems are attached, and adds `apply_deferred` so that commands are flushed after building the associated prefabs.

Then, individual blueprints can be defined by attaching a `BlueprintPlugin` for each pair of types that needs to be managed. `BlueprintPlugin` accepts three type parameters:

1. The type that will serve as the blueprint (which does not need to be a component but can be)
2. The `Bundle` that should be spawned
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

## Caution

Beware that this calls `despawn_recursive` and `despawn_descendants` to handle cleanup, so attaching child entities that aren't related to any blueprint is probably going to cause problems.
