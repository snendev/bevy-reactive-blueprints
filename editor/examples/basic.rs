use bevy::{
    color::palettes,
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    ecs::system::{StaticSystemParam, SystemParam},
    prelude::*,
};
use bevy_editor_pls::{
    editor::EditorInternalState, egui_dock::NodeIndex, prelude::NotInScene, AddEditorWindow,
    EditorPlugin,
};
use bevy_reactive_blueprints::{AsChild, BlueprintPlugin, BlueprintsPlugin, FromBlueprint};

use bevy_reactive_blueprints_editor_window::BlueprintSceneWindow;
use bevy_reactive_blueprints_editor_window::*;

pub fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..Default::default()
        }),
        EditorPlugin::default().in_new_window(Window::default()),
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        BlueprintsPlugin,
    ));

    // add the window
    app.add_editor_window::<BlueprintSceneWindow>();
    let mut internal_state = app.world_mut().resource_mut::<EditorInternalState>();
    internal_state.split_below::<BlueprintSceneWindow>(NodeIndex::root().left().left(), 0.6);

    // register some blueprint
    app.add_plugins(BlueprintPlugin::<RectBlueprint, RectBundle>::default())
        .add_plugins(BlueprintPlugin::<RectBlueprint, RectBundle, AsChild>::default())
        .register_blueprint::<RectBlueprint>()
        .register_type::<RectSize>();

    app.world_mut().spawn((
        NotInScene,
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::Z * 10.),
            ..Default::default()
        },
    ));

    app.run();
}

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
            color: Color::Srgba(palettes::css::BLUE),
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
    name: Name,
    rect: Rect,
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
            name: Name::new("My Rect"),
            size: RectSize(blueprint.size),
            rect: Rect,
            pbr: PbrBundle {
                mesh: params
                    .meshes
                    .add(Cuboid::new(blueprint.size.x, blueprint.size.y, 1.0).mesh()),
                material: params.materials.add(blueprint.color),
                transform: Transform::from_xyz(blueprint.origin.x, blueprint.origin.y, 0.0),
                ..default()
            },
        }
    }
}
