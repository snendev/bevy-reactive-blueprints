use std::path::{Path, PathBuf};

use bevy::{prelude::*, utils::HashMap};
use bevy_editor_pls::{
    default_windows::add::{AddItem, AddWindow},
    editor::Editor,
    editor_window::{EditorWindow, EditorWindowContext},
    egui_dock::egui,
    prelude::NotInScene,
};
use bevy_reactive_blueprints::Blueprint;

pub enum EditorOpenSetting {
    Windowed,
    FullScreen,
}

const ASSETS_PATH: &str = "editor-scenes";
const DEFAULT_FILENAME: &str = "scene";
const EXTENSION: &str = "scn.ron";

#[derive(Default)]
pub struct BlueprintSceneWindowState {
    filename: String,
    play_scene_request:
        Option<Result<Handle<DynamicScene>, Box<dyn std::error::Error + Send + Sync>>>,
    scene_save_result: Option<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
}

pub struct BlueprintSceneWindow;

impl EditorWindow for BlueprintSceneWindow {
    type State = BlueprintSceneWindowState;
    const NAME: &'static str = "Blueprint Scenes";

    fn ui(world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
        let state = cx.state_mut::<BlueprintSceneWindow>().unwrap();

        let assets_path = std::path::Path::new("assets").join(&ASSETS_PATH);
        // TODO: path pulls files from a different cwd than asset server
        let directory = std::fs::read_dir(&assets_path).unwrap_or_else(|_| {
            std::fs::create_dir(&assets_path).unwrap();
            std::fs::read_dir(&assets_path).unwrap()
        });

        ui.horizontal(|ui| {
            let res = egui::TextEdit::singleline(&mut state.filename)
                .hint_text(DEFAULT_FILENAME)
                .desired_width(120.0)
                .show(ui);

            if res.response.changed() {
                state.scene_save_result = None;
            }

            if ui.button("Save").clicked() {
                let filename = if state.filename.is_empty() {
                    DEFAULT_FILENAME
                } else {
                    &state.filename
                };
                let filename = std::path::Path::new(&assets_path)
                    .join(filename)
                    .with_extension(EXTENSION);

                let mut query = world.query_filtered::<Entity, Without<NotInScene>>();
                let entities = query.iter(world).collect();
                state.scene_save_result =
                    Some(save_world(world, filename.to_str().unwrap(), entities));
            }
        });

        if let Some(status) = &state.scene_save_result {
            match status {
                Ok(()) => {
                    ui.label(egui::RichText::new("Success!").color(egui::Color32::GREEN));
                }
                Err(error) => {
                    ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
                }
            }
        }

        for entry in directory {
            let entry = entry.unwrap();

            ui.horizontal(|ui| {
                let path = entry.path();
                // <etc>/filename.scn.ron
                let stripped_path = path.with_extension("").with_extension("");
                let file_stem = stripped_path.file_name().unwrap().to_str().unwrap();

                ui.label(file_stem);
                if ui.button("Play").clicked() {
                    // despawn the previous scene
                    type NotRelevant = (Without<NotInScene>, Without<Window>);
                    let mut query = world.query_filtered::<Entity, NotRelevant>();
                    for entity in query.iter(world).collect::<std::collections::HashSet<_>>() {
                        // TODO Some sort of despawn bug?
                        world.despawn(entity);
                    }
                    // load the new scene
                    let scene_filename = Path::new(&ASSETS_PATH)
                        .join(file_stem)
                        .with_extension(EXTENSION);
                    state.play_scene_request = Some(load_scene(
                        world,
                        scene_filename
                            .to_str()
                            .expect("Scene filename to be a valid file"),
                    ));
                }
            });
        }
        if let Some(status) = &state.play_scene_request {
            match status {
                Ok(scene) => {
                    if poll_loading_scene(world, scene).is_ok() {
                        state.play_scene_request = None;
                    }
                }
                Err(error) => {
                    ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
                }
            }
        }
    }
}

type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Resource)]
struct BlueprintsFilter(SceneFilter);

fn save_world(
    world: &World,
    name: &str,
    entities: std::collections::HashSet<Entity>,
) -> AnyResult<()> {
    let type_registry = world.get_resource::<AppTypeRegistry>().unwrap();
    let blueprints_filter = world.get_resource::<BlueprintsFilter>().unwrap();

    let mut scene_builder =
        DynamicSceneBuilder::from_world(world).with_filter(blueprints_filter.0.clone());
    scene_builder = scene_builder
        .extract_entities(entities.into_iter())
        .remove_empty_entities();
    let scene = scene_builder.build();

    let ron = scene.serialize_ron(type_registry)?;
    std::fs::write(name, ron)?;
    Ok(())
}

fn load_scene(world: &mut World, name: &str) -> AnyResult<Handle<DynamicScene>> {
    let asset_server = world.resource::<AssetServer>();
    let scene: Handle<DynamicScene> = asset_server.load(name.to_string());
    Ok(scene)
}

fn poll_loading_scene(world: &mut World, scene: &Handle<DynamicScene>) -> AnyResult<()> {
    world.resource_scope(
        |world: &mut World, scenes: Mut<Assets<DynamicScene>>| -> AnyResult<()> {
            let scene = match scenes.get(scene) {
                Some(scene) => Ok(scene),
                None => Err("Not ready yet!"),
            }?;
            world.resource_scope(|world: &mut World, registry: Mut<AppTypeRegistry>| {
                Ok(scene.write_to_world_with(world, &mut HashMap::default(), &registry)?)
            })
        },
    )
}

pub trait AppBlueprintExt {
    fn register_blueprint<B>(self) -> Self
    where
        B: Default + TypePath + Send + Sync + 'static;

    fn ignore_all_component<C>(self) -> Self
    where
        C: Component + Send + Sync + 'static;
}

impl AppBlueprintExt for &mut App {
    fn register_blueprint<B>(self) -> Self
    where
        B: Default + TypePath + Send + Sync + 'static,
    {
        let mut editor = self
            .world
            .get_resource_mut::<Editor>()
            .expect("Editor should exist");
        let state = editor
            .window_state_mut::<AddWindow>()
            .expect("AddWindow should exist");
        state.add(
            "Blueprints",
            AddItem::component_named::<Blueprint<B>>(B::type_path().into()),
        );
        let mut filter = self
            .world
            .get_resource_or_insert_with(|| BlueprintsFilter(SceneFilter::deny_all()));
        filter.0 = filter.0.clone().allow::<Blueprint<B>>();

        self
    }

    fn ignore_all_component<C>(self) -> Self
    where
        C: Component + Send + Sync + 'static,
    {
        self.add_systems(
            PostUpdate,
            |query: Query<Entity, Added<C>>, children: Query<&Children>, mut commands: Commands| {
                for entity in query.iter() {
                    let mut ignored_hierarchy = vec![];
                    collect_hierarchy(&mut ignored_hierarchy, entity, &children);
                    for entity in ignored_hierarchy {
                        commands.entity(entity).insert(NotInScene);
                    }
                }
            },
        );

        self
    }
}

fn collect_hierarchy(buffer: &mut Vec<Entity>, entity: Entity, children_query: &Query<&Children>) {
    buffer.push(entity);
    if let Ok(children) = children_query.get(entity) {
        for child in children {
            collect_hierarchy(buffer, *child, children_query);
        }
    }
}
