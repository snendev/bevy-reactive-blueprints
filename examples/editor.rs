use bevy::{
    ecs::system::{StaticSystemParam, SystemParam},
    prelude::*,
};
use bevy_editor_pls::{
    default_windows::add::{AddItem, AddWindow},
    editor::Editor,
    prelude::*,
};
use bevy_reactive_blueprints::*;

#[derive(Reflect)]
struct RectBlueprint {
    origin: Vec2,
    size: Vec2,
    color: Color,
}

impl Default for RectBlueprint {
    fn default() -> Self {
        RectBlueprint {
            size: 4. * Vec2::ONE,
            color: Color::BLUE,
            origin: Default::default(),
        }
    }
}

#[derive(Reflect)]
struct RectHierarchalBlueprint {
    origin: Vec2,
    size: Vec2,
    color: Color,
}

impl Default for RectHierarchalBlueprint {
    fn default() -> Self {
        RectHierarchalBlueprint {
            size: 4. * Vec2::ONE,
            color: Color::BLUE,
            origin: Default::default(),
        }
    }
}

// marker component useful for queries
#[derive(Component)]
struct Rect;

// additional component useful for systems
#[derive(Clone, Component, Default, Reflect)]
#[reflect(Component)]
struct RectSize(Vec2);

#[derive(Bundle)]
struct RectBundle {
    size: RectSize,
    pbr: PbrBundle,
}

#[derive(SystemParam)]
struct RectBlueprintParams<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl FromBlueprint<RectBlueprint> for RectBundle {
    type Params<'w, 's> = RectBlueprintParams<'w>;
    fn from_blueprint(
        blueprint: &RectBlueprint,
        params: &mut StaticSystemParam<Self::Params<'_, '_>>,
    ) -> Self {
        RectBundle {
            size: RectSize(blueprint.size),
            pbr: PbrBundle {
                mesh: params
                    .meshes
                    .add(shape::Box::new(blueprint.size.x, blueprint.size.y, 1.0).into()),
                material: params.materials.add(blueprint.color.into()),
                transform: Transform::from_xyz(blueprint.origin.x, blueprint.origin.y, 0.0),
                ..default()
            },
        }
    }
}

impl FromBlueprint<RectHierarchalBlueprint> for RectBundle {
    type Params<'w, 's> = RectBlueprintParams<'w>;
    fn from_blueprint(
        blueprint: &RectHierarchalBlueprint,
        params: &mut StaticSystemParam<Self::Params<'_, '_>>,
    ) -> Self {
        RectBundle {
            size: RectSize(blueprint.size),
            pbr: PbrBundle {
                mesh: params
                    .meshes
                    .add(shape::Box::new(blueprint.size.x, blueprint.size.y, 1.0).into()),
                material: params.materials.add(blueprint.color.into()),
                transform: Transform::from_xyz(blueprint.origin.x, blueprint.origin.y, 0.0),
                ..default()
            },
        }
    }
}

// Use this to register blueprints to the editor
fn register_blueprints(world: &mut World) {
    let mut editor = world
        .get_resource_mut::<Editor>()
        .expect("Editor should exist");
    let state = editor
        .window_state_mut::<AddWindow>()
        .expect("AddWindow should exist");
    state.add(
        "Blueprints",
        AddItem::component_named::<Blueprint<RectBlueprint>>("Rect".into()),
    );
    state.add(
        "Blueprints",
        AddItem::component_named::<Blueprint<RectHierarchalBlueprint>>(
            "Rect (Uses Children)".into(),
        ),
    );
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, EditorPlugin::default(), BlueprintsPlugin));
    app.add_plugins(BlueprintPlugin::<RectBlueprint, RectBundle>::default());
    app.add_plugins(BlueprintPlugin::<
        RectHierarchalBlueprint,
        RectBundle,
        AsChild,
    >::default());
    register_blueprints(&mut app.world);
    app.run();
}
