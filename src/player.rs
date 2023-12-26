use crate::map::{MapSettings, TileDigging, TileType};
use crate::prelude::*;

pub struct PlayerPlugin;

#[derive(Component, Default)]
pub struct Player;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MainStates::InGame), spawn_player)
            .add_systems(Update, move_player.run_if(in_state(MainStates::InGame)));
    }
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    player: Player,
    sprite_bundle: SpriteBundle,
    position: GridPosition,
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_settings: Res<MapSettings>,
) {
    let position = GridPosition(TilePos::new(
        map_settings.size.0 / 2,
        map_settings.size.1 / 2,
    ));

    let sprite = SpriteBundle {
        texture: asset_server.load("digger.png"),
        transform: Transform::from_xyz(
            position.x as f32 * map_settings.tile_size,
            position.y as f32 * map_settings.tile_size,
            0.0,
        ),
        ..default()
    };

    println!("{:?}", position);
    commands.spawn(PlayerBundle {
        sprite_bundle: sprite,
        position,
        ..default()
    });
}

fn move_player(
    mut commands: Commands,
    player_query: Query<(Entity, &GridPosition), Without<MoveTo>>,
    map_query: Query<(&TilemapSize, &TileStorage)>,
    tile_query: Query<&TileType>,
    input: Res<Input<KeyCode>>,
) {
    let (map_size, tiles) = map_query.single();
    if let Ok((e, grid_pos)) = player_query.get_single() {
        let mut move_target = IVec2::ZERO;

        if input.pressed(KeyCode::D) || input.pressed(KeyCode::Right) {
            move_target = IVec2::new(1, 0);
        } else if input.pressed(KeyCode::S) || input.pressed(KeyCode::Down) {
            move_target = IVec2::new(0, -1);
        } else if input.pressed(KeyCode::A) || input.pressed(KeyCode::Left) {
            move_target = IVec2::new(-1, 0);
        } else if input.pressed(KeyCode::W) || input.pressed(KeyCode::Up) {
            move_target = IVec2::new(0, 1);
        }

        if move_target != IVec2::ZERO {
            let new_pos = TilePos::from_i32_pair(
                grid_pos.x as i32 + move_target.x,
                grid_pos.y as i32 + move_target.y,
                &map_size,
            );

            if let Some(new_pos) = new_pos {
                let tile_entity = tiles.get(&new_pos).expect("Tile entity should exist!");

                let move_speed = match tile_query
                    .get(tile_entity)
                    .expect("Tile should have a tile type")
                {
                    TileType::Walkable => 0.5,
                    TileType::Diggable { hardness, .. } => {
                        let time = 0.5 * (1.0 + hardness);
                        commands.entity(tile_entity).insert(TileDigging::new(time));
                        time
                    }
                    TileType::Impassable => return,
                };

                commands.entity(e).insert(MoveTo::new(new_pos, move_speed));
            }
        }
    }
}
