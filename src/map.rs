use bevy::asset::LoadState;
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::Neighbors;
use bevy_ecs_tilemap::prelude::*;
use std::collections::HashMap;

use crate::prelude::*;

pub struct MapPlugin;

#[derive(serde::Deserialize, Component, Debug, Clone, PartialEq)]
pub enum TileType {
    Walkable,
    Diggable { level: u32, hardness: f32 },
    Impassable,
}

#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct TileDigging(Timer);

impl TileDigging {
    pub fn new(time: f32) -> Self {
        TileDigging(Timer::from_seconds(time, TimerMode::Once))
    }
}

#[derive(Bundle)]
pub struct GameTileBundle {
    tile_bundle: TileBundle,
    tile_type: TileType,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TileData {
    index: u32,
    tile_type: TileType,
}

#[derive(
    Clone, serde::Deserialize, bevy::asset::Asset, bevy::reflect::TypePath, Debug, Deref, Resource,
)]
pub struct TileInfo {
    #[deref]
    tiles: HashMap<String, TileData>,
}

impl TileInfo {
    pub fn create_bundle(
        &self,
        name: &str,
        position: TilePos,
        tilemap_id: TilemapId,
        visible: bool,
    ) -> Option<GameTileBundle> {
        let data = self.tiles.get(name)?;

        Some(GameTileBundle {
            tile_bundle: TileBundle {
                position,
                texture_index: TileTextureIndex(data.index),
                tilemap_id,
                visible: TileVisible(visible),
                ..default()
            },
            tile_type: data.tile_type.clone(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum MapStates {
    #[default]
    LoadingAssets,
    Ready,
    Generated,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct MapSettings {
    pub size: (u32, u32),
    pub tile_size: f32,
}

#[derive(Resource)]
pub struct MapAssets {
    pub texture: Handle<Image>,
    pub tile_info: Handle<TileInfo>,
}

fn update_tile_digging(
    mut commands: Commands,
    mut digging_query: Query<(Entity, &mut TileDigging, &TilePos, &TilemapId)>,
    tile_info: Res<TileInfo>,
    time: Res<Time>,
    mut tile_storage: Query<&mut TileStorage>,
) {
    for (e, mut dig, tile_pos, tilemap_id) in digging_query.iter_mut() {
        dig.tick(time.delta());
        if dig.finished() {
            let mut tile_storage = tile_storage.single_mut();
            commands.entity(e).despawn();

            let tile_bundle = tile_info
                .create_bundle("ground", tile_pos.clone(), tilemap_id.clone(), true)
                .unwrap();
            let new_entity = commands.spawn(tile_bundle).id();
            tile_storage.set(tile_pos, new_entity);
        }
    }
}

fn load_map_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(MapAssets {
        texture: asset_server.load("tilemap.png"),
        tile_info: asset_server.load("tiles.info.ron"),
    });
}

fn get_group_load_state(
    asset_server: Res<AssetServer>,
    handles: impl IntoIterator<Item = UntypedHandle>,
) -> LoadState {
    let mut load_state = LoadState::Loaded;

    for handle in handles {
        match asset_server.get_load_state(handle.id()) {
            Some(state) => match state {
                LoadState::Loaded => continue,
                LoadState::Loading => load_state = LoadState::Loading,
                LoadState::Failed => return LoadState::Failed,
                LoadState::NotLoaded => return LoadState::NotLoaded,
            },
            None => return LoadState::NotLoaded,
        }
    }

    load_state
}

fn check_map_asset_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<MapStates>>,
    map_assets: Res<MapAssets>,
    tile_info_assets: Res<Assets<TileInfo>>,
) {
    let assets = vec![
        map_assets.texture.clone().untyped(),
        map_assets.tile_info.clone().untyped(),
    ];

    match get_group_load_state(asset_server, assets) {
        LoadState::Loaded => {
            next_state.set(MapStates::Ready);
            let tile_info = tile_info_assets
                .get(map_assets.tile_info.clone())
                .expect("TileInfo should be loaded!");

            commands.insert_resource(tile_info.clone());
        }
        _ => {}
    }
}

fn update_visibility(
    changed_query: Query<(Entity, &TilePos, &TileType), Changed<TileType>>,
    mut tile_query: Query<&mut TileVisible>,
    tile_storage: Query<(&TileStorage, &TilemapSize)>,
) {
    let (tile_storage, map_size) = tile_storage.single();

    for (e, tile_pos, tile_type) in changed_query.iter() {
        let visible = tile_query.get(e).unwrap().clone();
        if visible.0 && *tile_type == TileType::Walkable {
            let neighbors = Neighbors::get_square_neighboring_positions(tile_pos, map_size, true);

            for neighbor in neighbors.iter() {
                let tile_entity = tile_storage.get(neighbor).unwrap();
                let mut tile_vis = tile_query.get_mut(tile_entity).unwrap();
                *tile_vis = visible.clone();
            }
        }
    }
}

fn create_map(
    mut commands: Commands,
    settings: Res<MapSettings>,
    map_assets: Res<MapAssets>,
    tile_info: Res<TileInfo>,
    mut next_map_state: ResMut<NextState<MapStates>>,
    mut next_main_state: ResMut<NextState<MainStates>>,
) {
    println!("Creating map");

    println!("{:?}", tile_info);

    let map_size = TilemapSize {
        x: settings.size.0,
        y: settings.size.1,
    };
    let tilemap_id = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let mut tile_bundle = tile_info
                .create_bundle("ice", tile_pos, TilemapId(tilemap_id), false)
                .unwrap();
            let tile_entity = commands.spawn(tile_bundle).id();
            tile_storage.set(&tile_pos, tile_entity)
        }
    }

    for x in (settings.size.0 / 2 - 1)..(settings.size.0 / 2 + 2) {
        for y in (settings.size.1 / 2 - 1)..(settings.size.1 / 2 + 2) {
            let tile_pos = TilePos { x, y };
            let tile_bundle = tile_info
                .create_bundle("ground", tile_pos, TilemapId(tilemap_id), true)
                .unwrap();
            let tile_entity = commands.spawn(tile_bundle).id();
            tile_storage.set(&tile_pos, tile_entity)
        }
    }

    let tile_size = TilemapTileSize {
        x: settings.tile_size,
        y: settings.tile_size,
    };
    let grid_size = tile_size.into();
    let map_type = TilemapType::Square;

    commands.entity(tilemap_id).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(map_assets.texture.clone()),
        tile_size,
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..default()
    });

    next_map_state.set(MapStates::Generated);
    next_main_state.set(MainStates::InGame);
}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .add_state::<MapStates>()
            .add_plugins(RonAssetPlugin::<TileInfo>::new(&["info.ron"]))
            .add_systems(Startup, load_map_assets)
            .add_systems(
                Update,
                check_map_asset_loading.run_if(in_state(MapStates::LoadingAssets)),
            )
            .add_systems(
                Update,
                update_tile_digging.run_if(in_state(MainStates::InGame)),
            )
            .add_systems(OnEnter(MapStates::Ready), create_map)
            .add_systems(
                Update,
                update_visibility.run_if(in_state(MainStates::InGame)),
            )
            .insert_resource(MapSettings {
                size: (31, 31),
                tile_size: 16.0,
            });
    }
}
