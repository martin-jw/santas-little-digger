use crate::prelude::*;
use bevy::ecs::query::QuerySingleError;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pixel_camera::{PixelCameraPlugin, PixelViewport, PixelZoom};

mod map;
mod player;

mod prelude {
    pub use bevy::prelude::*;
    pub use bevy_ecs_tilemap::prelude::*;

    pub use super::GridPosition;
    pub use super::MainStates;
    pub use super::MoveTo;
}

#[derive(Hash, Clone, Debug, PartialEq, Eq, States, Default)]
pub enum MainStates {
    #[default]
    Loading,
    InGame,
}

#[derive(Component, Deref, DerefMut, Default, Debug)]
pub struct GridPosition(TilePos);

/// Component for signaling that an entity with a GridPosition
/// should move to the specified grid position.
#[derive(Component)]
pub struct MoveTo {
    target: TilePos,
    elapsed_time: f32,
    movement_time: f32,
}

impl MoveTo {
    fn new(target: TilePos, movement_time: f32) -> Self {
        MoveTo {
            target,
            movement_time,
            elapsed_time: 0.0,
        }
    }
}

fn animate_moveto(
    mut commands: Commands,
    mut moveto_query: Query<(Entity, &mut Transform, &mut GridPosition, &mut MoveTo)>,
    tilemap_query: Query<(&TilemapGridSize, &TilemapType)>,
    time: Res<Time>,
) {
    let (grid_size, map_type) = tilemap_query.single();
    for (e, mut transform, mut grid_pos, mut move_to) in moveto_query.iter_mut() {
        move_to.elapsed_time += time.delta_seconds();
        if move_to.elapsed_time >= move_to.movement_time {
            // Movement is done, update grid position and
            // remove MoveTo component.
            grid_pos.0 = move_to.target;
            transform.translation = grid_pos.center_in_world(grid_size, map_type).extend(0.0);
            commands.entity(e).remove::<MoveTo>();
        } else {
            let start = grid_pos.center_in_world(grid_size, map_type);
            let end = move_to.target.center_in_world(grid_size, map_type);
            let pos = start.lerp(end, move_to.elapsed_time / move_to.movement_time);
            transform.translation = pos.extend(0.0);
        }
    }
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        MainCamera,
        PixelZoom::Fixed(4),
        PixelViewport,
    ));
}

fn camera_follow_player(
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    player_query: Query<&Transform, (With<player::Player>, Without<MainCamera>)>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        match camera_query.get_single_mut() {
            Ok(mut camera_transform) => {
                camera_transform.translation = player_transform
                    .translation
                    .xy()
                    .extend(camera_transform.translation.z);
            }
            Err(QuerySingleError::MultipleEntities(_)) => {
                panic!("There is more than one MainCamera, this should not happen!")
            }
            _ => {}
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(player::PlayerPlugin)
        .add_plugins(PixelCameraPlugin)
        .add_plugins(map::MapPlugin)
        .add_plugins(WorldInspectorPlugin::new())
        .add_state::<MainStates>()
        .insert_resource(Msaa::Off)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, camera_follow_player)
        .add_systems(FixedUpdate, animate_moveto)
        .run();
}
