use std::{f32::consts::PI, time::Duration};

use bevy::{
    app::AppExit,
    asset::ChangeWatcher,
    audio::{PlaybackMode, Volume, VolumeLevel},
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    math::{vec2, vec3, Vec3Swizzles},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        camera::ScalingMode,
        render_resource::{AddressMode, AsBindGroup, SamplerDescriptor, ShaderRef},
    },
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    time::Stopwatch,
    window::PrimaryWindow,
};
use button::{interact_button, ButtonCommands};
// use mute::MuteButtonPlugin;

#[cfg(feature = "dev")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use utils::AssetHandle;

mod button;
// mod mute;
mod utils;

static PRIMARY_COLOR_HUE: f32 = 0.59;
static MENU_MUSIC_VOLUME: f32 = 0.5;
static PLAYING_MUSIC_VOLUME: f32 = 0.8;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    // mode: WindowMode::Fullscreen,
                    // present_mode: PresentMode::AutoNoVsync,
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(1000)),
                ..Default::default()
            })
            .set(ImagePlugin {
                default_sampler: SamplerDescriptor {
                    address_mode_u: AddressMode::Repeat,
                    address_mode_v: AddressMode::Repeat,
                    address_mode_w: AddressMode::Repeat,
                    ..Default::default()
                },
            }),
    )
    .insert_resource(ClearColor(Color::hsl(PRIMARY_COLOR_HUE * 360.0, 0.2, 0.1)))
    .insert_resource(Score(0))
    .insert_resource(Level(3))
    .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
    .insert_resource(LaunchPower(Stopwatch::new()))
    .insert_resource(PrimaryColorHue(PRIMARY_COLOR_HUE))
    .add_plugins(MaterialPlugin::<SunMaterial>::default())
    .add_plugins(MaterialPlugin::<BackgroundMaterial>::default())
    // .add_plugins(MuteButtonPlugin)
    .add_state::<GameState>()
    .add_systems(Startup, setup)
    .add_systems(OnEnter(GameState::Menu), on_enter_menu)
    .add_systems(OnEnter(GameState::Launched), on_enter_launched)
    .add_systems(OnEnter(GameState::ChargingLaunch), on_enter_charging)
    .add_systems(OnExit(GameState::Menu), (on_exit_menu, on_enter_playing))
    .add_systems(OnEnter(GameState::ReadyToLaunch), on_enter_ready)
    // .add_systems(OnExit(GameState::ReadyToLaunch), on_exit_playing)
    .add_systems(FixedUpdate, (apply_gravity))
    .add_systems(
        Update,
        (increase_crate_mass, orbit_debris, remove_crate_on_sun_collision,).run_if(in_state(GameState::Launched)),
    )
    .add_systems(
        Update,
        (
            exit_on_esc.run_if(is_desktop),
            interact_button,
            always,
            spin_earth,
            spin_debris,
            // spin_crates,
            update_cannon_transform,
            rotate_crates,
            apply_velocity,
            // apply_gravity,
            // remove_crate_on_sun_collision,
            remove_crate_on_earth_collision,
            fade_explosions,
            update_camera_position,
            attach_debris_to_crate_collision,
            update_music_speed,
        ),
    )
    .add_systems(
        Update,
        (start_launching, orbit_debris).run_if(in_state(GameState::ReadyToLaunch)),
    )
    .add_systems(
        Update,
        (update_launch_power, launch, orbit_debris).run_if(in_state(GameState::ChargingLaunch)),
    )
    .add_systems(
        Update,
        (interact_play_button,).run_if(in_state(GameState::Menu)),
    )
    .add_systems(
        Update,
        (while_playing,).run_if(in_state(GameState::ReadyToLaunch)),
    );

    #[cfg(feature = "dev")]
    app.add_plugins(WorldInspectorPlugin::new());

    app.run();
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    Menu,
    ReadyToLaunch,
    ChargingLaunch,
    Launched,
}

#[derive(Component)]
struct PlayButton;

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PrimaryColorHue(f32);

fn is_desktop() -> bool {
    std::env::consts::OS == "macos" || std::env::consts::OS == "windows"
}

#[derive(Component)]
struct Music;

#[derive(Component)]
struct ChargeSound;

#[derive(Component)]
struct FireSound;

#[derive(Component)]
struct EarthDestroyedSound;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct Explosion;

// #[derive(Resource)]
// struct ExplosionMesh(Option<Handle<Mesh>>);

// #[derive(Resource)]
// struct ExplosionMaterial(Option<Handle<ColorMaterial>>);

#[derive(Component)]
struct WhiningSound;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut sun_materials: ResMut<Assets<SunMaterial>>,
    mut bg_materials: ResMut<Assets<BackgroundMaterial>>,
    asset_server: Res<AssetServer>,
    level: Res<Level>,
) {
    // whining
    commands.spawn((
        AudioBundle {
            source: asset_server.load("aaa.ogg"),
            settings: PlaybackSettings {
                paused: false,
                mode: PlaybackMode::Loop,
                volume: Volume::Relative(VolumeLevel::new(0.0)),
                ..default()
            },
            ..default()
        },
        WhiningSound,
    ));

    // spawn score text
    commands.spawn((
        ScoreText,
        TextBundle::from_section(
            format!("Level {}", level.0),
            TextStyle {
                font_size: 64.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            margin: UiRect::new(Val::Auto, Val::Auto, Val::Vh(20.0), Val::Auto),
            ..default()
        }),
    ));

    // music
    commands.spawn((
        AudioBundle {
            source: asset_server.load("music.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Relative(VolumeLevel::new(MENU_MUSIC_VOLUME)),
                ..default()
            },
            ..default()
        },
        Music,
    ));

    // // charge sound
    // commands.spawn((
    //     AudioBundle {
    //         source: asset_server.load("charge.ogg"),
    //         settings: PlaybackSettings {
    //             mode: PlaybackMode::Despawn,
    //             volume: Volume::Relative(VolumeLevel::new(0.4)),
    //             paused: true,
    //             ..default()
    //         },
    //         ..default()
    //     },
    //     ChargeSound,
    // ));

    // // fire sound
    // commands.spawn((
    //     AudioBundle {
    //         source: asset_server.load("fired.ogg"),
    //         settings: PlaybackSettings {
    //             mode: PlaybackMode::Once,
    //             volume: Volume::Relative(VolumeLevel::new(0.4)),
    //             paused: true,
    //             ..default()
    //         },
    //         ..default()
    //     },
    //     FireSound,
    // ));

    // // earth destroyed sound
    // commands.spawn((
    //     AudioBundle {
    //         source: asset_server.load("earth_destroyed.ogg"),
    //         settings: PlaybackSettings {
    //             mode: PlaybackMode::Once,
    //             volume: Volume::Relative(VolumeLevel::new(0.4)),
    //             paused: true,
    //             ..default()
    //         },
    //         ..default()
    //     },
    //     EarthDestroyedSound,
    // ));

    // camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            tonemapping: Tonemapping::None,
            projection: Projection::Orthographic(OrthographicProjection {
                viewport_origin: vec2(0.5, 0.5),
                scaling_mode: ScalingMode::FixedVertical(720.0),
                scale: 0.12,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings {
            intensity: 0.14,
            ..default()
        },
    ));

    // AssetHandle example
    // commands.insert_resource(AssetHandle::<Circle, ColorMaterial>::new(
    //     materials.add(Color::hsl((PRIMARY_COLOR_HUE - 0.5) * 360.0, 0.7, 0.8).into()),
    // ));

    commands.insert_resource(AssetHandle::<Debris, Scene>::new(
        asset_server.load("debris.glb#Scene0"),
    ));

    // explosion asset handles
    commands.insert_resource(AssetHandle::<Explosion, Mesh>::new(
        meshes.add(shape::Circle::new(1.0).into()).into(),
    ));
    commands.insert_resource(AssetHandle::<Explosion, StandardMaterial>::new(
        std_materials.add(StandardMaterial {
            base_color: Color::ORANGE_RED * 20.0,
            unlit: true,
            ..default()
        }),
    ));

    // // spawn stars.png texture
    // commands.spawn(MaterialMeshBundle {
    //     mesh: meshes.add(shape::Plane::from_size(100.0).into()).into(),
    //     material: std_materials.add(asset_server.load("stars.png").into()),
    //     transform: Transform::from_translation(vec3(0.0, 0.0, -100.0))
    //         .with_rotation(Quat::from_rotation_x(PI / 2.0)),
    //     ..default()
    // });

    // spawn background
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Plane::from_size(1000.0).into()).into(),
        material: bg_materials.add(BackgroundMaterial {
            color: Color::WHITE,
            color_texture: asset_server.load("stars.png"),
        }),
        transform: Transform::from_translation(vec3(0.0, 0.0, -100.0))
            .with_rotation(Quat::from_rotation_x(PI / 2.0)),
        ..default()
    });

    // spawn sun
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(shape::Plane::from_size(30.0).into()).into(),
            material: sun_materials.add(SunMaterial {
                color: Color::ORANGE_RED,
                color_texture: asset_server.load("noise.png"),
            }),
            transform: Transform::from_translation(vec3(0.0, 15.0, -50.0))
                .with_rotation(Quat::from_rotation_x(PI / 2.0)),
            ..default()
        },
        Sun,
    ));

    // spawn point light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(vec3(0.0, 15.0, 0.0)),
        point_light: PointLight {
            intensity: 1000000.0,
            range: 1000.0,
            color: Color::rgb(1.0, 0.8, 0.5),
            ..default()
        },
        ..default()
    });

    // spawn earth
    commands
        .spawn(SceneBundle {
            scene: asset_server.load("earth.glb#Scene0"),
            transform: Transform::from_xyz(0.0, -25.0, -20.0)
                .with_scale(Vec3::splat(5.0))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0)),
            ..Default::default()
        })
        .insert(Earth);

    // // spawn explosion
    // commands
    //     .spawn((PbrBundle {
    //         mesh: meshes.add(shape::Circle::new(5.0).into()).into(),
    //         material: std_materials.add(StandardMaterial {
    //             base_color: Color::ORANGE_RED * 20.0,
    //             unlit: true,
    //             ..default()
    //         }),
    //         transform: Transform::from_xyz(0.0, -25.0, 1.0),
    //         ..default()
    //     }, Explosion));

    // spawn cannon + crate
    commands.spawn((
        Cannon,
        SceneBundle {
            scene: asset_server.load("launcher.glb#Scene0"),
            transform: Transform::from_xyz(0.0, 0.0, 0.0)
                .with_scale(Vec3::splat(1.0))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0)),
            ..default()
        },
        // SpatialBundle {
        //     transform: Transform::from_translation(vec3(0.0, 0.0, 0.0)),
        //     ..default()
        // },
    ));
    // .with_children(|parent| {
    //     parent.spawn((
    //         SceneBundle {
    //             scene: asset_server.load("crate.glb#Scene0"),
    //             transform: Transform::from_xyz(0.0, 3.0, 0.0)
    //                 .with_scale(Vec3::splat(1.0))
    //                 .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0)),
    //             ..default()
    //         },
    //         Crate,
    //         Mass(1.0),
    //         CurrentCrate,
    //     ));
    // });
}

// debris orbit the sun
fn orbit_debris(
    time: Res<Time>,
    mut q_debris: Query<&mut Transform, (With<Debris>, Without<PickedUp>)>,
    q_sun: Query<&Transform, (With<Sun>, Without<Debris>)>,
    level: Res<Level>,
) {
    return; // Spinning debris = free win.. :P

    let speed = ((level.0 as f32 - 3.0).max(0.0) * 0.1).min(0.3);

    for mut transform in q_debris.iter_mut() {
        if let Ok(sun_transform) = q_sun.get_single() {
            let sun_pos = sun_transform.translation.xy();
            let debris_pos = transform.translation.xy();
            let distance = sun_pos.distance(debris_pos);
            let direction = (sun_pos - debris_pos).normalize();
            let direction_angle = direction.y.atan2(direction.x);

            // move position along direction_angle
            let new_direction_angle = direction_angle + time.delta_seconds() * speed;
            let new_direction = Vec2::new(new_direction_angle.cos(), new_direction_angle.sin());
            let new_position = sun_pos - new_direction * distance;
            transform.translation = vec3(new_position.x, new_position.y, 0.0);
        }
    }
}

fn increase_crate_mass(mut q_crate: Query<&mut Mass, With<Crate>>, time: Res<Time>) {
    for mut mass in q_crate.iter_mut() {
        mass.0 += time.delta_seconds() * 0.35;
    }
}

// scale down explosions in size, and remove when small enough
fn fade_explosions(
    time: Res<Time>,
    mut commands: Commands,
    mut q_explosion: Query<(Entity, &mut Transform), With<Explosion>>,
) {
    for (ent, mut transform) in q_explosion.iter_mut() {
        let scale = transform.scale;

        let n = time.delta_seconds() * 2.0;
        let new_scale = scale.lerp(Vec3::ZERO, n);

        transform.scale = new_scale;
        if new_scale.x < 0.01 {
            commands.entity(ent).despawn_recursive();
        }
    }
}

fn on_enter_ready(
    mut commands: Commands,
    q_cannon: Query<Entity, With<Cannon>>,
    asset_server: Res<AssetServer>,
) {
    // spawn crate in cannon
    for cannon_ent in q_cannon.iter() {
        commands.entity(cannon_ent).with_children(|parent| {
            parent.spawn((
                SceneBundle {
                    scene: asset_server.load("crate.glb#Scene0"),
                    transform: Transform::from_xyz(0.0, 3.0, 0.0)
                        .with_scale(Vec3::splat(1.0))
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0)),
                    ..default()
                },
                Crate,
                Mass(0.5),
                CurrentCrate,
            ));
        });
    }
}

#[derive(Component)]
struct Earth;

#[derive(Component)]
struct Debris;

#[derive(Component)]
struct Sun;

#[derive(Component)]
struct Cannon;

#[derive(Component)]
struct Crate;

#[derive(Component)]
struct CurrentCrate;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Mass(f32);

// if in state ReadyToLaunch & LMB pressed, go to ChargingLaunch
fn start_launching(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut launch_power: ResMut<LaunchPower>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        next_state.set(GameState::ChargingLaunch);

        // reset launch_power
        launch_power.0.reset();
    }
}

fn update_launch_power(
    mut launch_power: ResMut<LaunchPower>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    launch_power.0.tick(time.delta());

    // if launch_power > 1.0, go to Launched
    if launch_power.0.elapsed_secs() > 2.0 {
        next_state.set(GameState::Launched);
    }
}

fn on_enter_charging(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioBundle {
            source: asset_server.load("charge.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Relative(VolumeLevel::new(0.4)),
                paused: false,
                ..default()
            },
            ..default()
        },
        ChargeSound,
    ));
}

fn on_enter_launched(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut launch_power: ResMut<LaunchPower>,
    mut current_crate: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (With<CurrentCrate>, Without<Cannon>, Without<Earth>),
    >,
    cannon: Query<&Transform, (With<Cannon>, Without<Earth>)>,
    earth: Query<&Transform, (With<Earth>, Without<Cannon>)>,
    charge_sound_controller: Query<(Entity, &AudioSink), With<ChargeSound>>,
    whining_controller: Query<&AudioSink, With<WhiningSound>>,
) {
    // set whining speed
    for sink in whining_controller.iter() {
        // sink.play();
        sink.set_speed(0.1);
        // sink.set_volume(0.9);
    }

    for (ent, sink) in charge_sound_controller.iter() {
        sink.stop();
        commands.entity(ent).despawn_recursive();
    }

    commands.spawn((
        AudioBundle {
            source: asset_server.load("fired.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Relative(VolumeLevel::new(0.4)),
                paused: false,
                ..default()
            },
            ..default()
        },
        FireSound,
    ));

    let (crate_ent, mut crate_transform, crate_global_transform) = current_crate.single_mut();
    let cannon_transform = cannon.single().clone();
    let earth_transform = earth.single().clone();
    let translation_diff = cannon_transform.translation - earth_transform.translation;
    let diff_normal = translation_diff.normalize().xy();

    // add Velocity to current crate
    let power = launch_power.0.elapsed_secs() * 1.35;
    commands
        .entity(crate_ent)
        .insert(Velocity(diff_normal * power));

    // add cannon translation to crate
    crate_transform.translation = crate_global_transform.translation();
    crate_transform.rotation = cannon_transform.rotation * crate_transform.rotation;

    // move current_crate from parent to root
    commands.entity(crate_ent).remove::<Parent>();

    // reset launch_power
    launch_power.0.reset();
}

// if in state ChargingLaunch & LMB not pressed, go to Launched
fn launch(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mouse_button_input: Res<Input<MouseButton>>,
    current_crate: Query<Entity, With<CurrentCrate>>,
    launch_power: Res<LaunchPower>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        next_state.set(GameState::Launched);
    }
}

fn apply_gravity(
    // time: Res<Time>,
    mut q_crate: Query<(&mut Velocity, &Transform, &Mass), With<Crate>>,
    q_sun: Query<&Transform, (With<Sun>, Without<Crate>)>,
    q_earth: Query<&Transform, (With<Earth>, Without<Cannon>, Without<Sun>)>,
) {
    for (mut velocity, crate_transform, mass) in q_crate.iter_mut() {
        for sun_transform in q_sun.iter() {
            let sun_pos = sun_transform.translation;
            let crate_pos = crate_transform.translation;
            let distance = sun_pos.distance(crate_pos);
            let direction = (sun_pos - crate_pos).normalize();
            let gravity = (direction * 100.0 * mass.0) / distance.powi(2);
            velocity.0 += Vec2::new(gravity.x, gravity.y);
        }

        for earth_transform in q_earth.iter() {
            let sun_pos = earth_transform.translation;
            let crate_pos = crate_transform.translation;
            let distance = sun_pos.distance(crate_pos);
            let direction = (sun_pos - crate_pos).normalize();
            let gravity = (direction * 2.3 * mass.0) / distance.powi(2);
            velocity.0 += Vec2::new(gravity.x, gravity.y);
        }
    }
}

fn apply_velocity(mut q_crate: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in q_crate.iter_mut() {
        transform.translation +=
            Vec3::new(velocity.0.x, velocity.0.y, 0.0) * time.delta_seconds() * 20.0;
    }
}

// based on cursor position, move cannon in an arc around earth
fn update_cannon_transform(
    mut q_cannon: Query<&mut Transform, With<Cannon>>,
    q_earth: Query<&Transform, (With<Earth>, Without<Cannon>)>,
    // mut mouse_pos: EventReader<CursorMoved>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    launch_power: Res<LaunchPower>,
) {
    let (camera, camera_transform) = camera_q.single();

    if let Ok(mut window) = primary_window.get_single() {
        let window_width = window.width() as f32;
        let window_x_center = window_width / 2.0;

        if let Some(cursor) = window.cursor_position() {
            for mut transform in q_cannon.iter_mut() {
                let earth_transform = q_earth.single();
                if let Some(cursor_world_pos) = window
                    .cursor_position()
                    .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
                    .map(|ray| ray.origin.truncate())
                {
                    let offset = cursor_world_pos - earth_transform.translation.xy();
                    let normal = offset.normalize();
                    let radius = 6.0;
                    let x = normal.x * radius;
                    let y = normal.y * radius;

                    let n = time.delta_seconds() * 16.0;

                    let current_translation = transform.translation;
                    let target_translation = Vec3::new(x, y, 0.0) + earth_transform.translation;
                    let current_rotation = transform.rotation;
                    //lookat
                    let target_rotation = Quat::from_rotation_z(-normal.x.atan2(normal.y));
                    let new_translation = current_translation.lerp(target_translation, n);
                    let new_rotation = current_rotation.lerp(target_rotation, n);

                    // also, rotate based on launch power
                    let power = launch_power.0.elapsed_secs() * 1.35;
                    let rotation = Quat::from_rotation_y(power * 0.5);
                    let new_rotation = new_rotation * rotation;

                    // also, scale horizontally based on launch power
                    let extra_width = power * 0.5;
                    let scale = Vec3::new(1.0 + extra_width, 1.0 - extra_width * 0.25, 1.0 + extra_width);

                    transform.translation = new_translation;
                    transform.rotation = new_rotation;
                    transform.scale = scale;
                }
            }
        }
    }
}

// fn update_cannon_transform(
//     mut q_cannon: Query<&mut Transform, With<Cannon>>,
//     q_earth: Query<&Transform, (With<Earth>, Without<Cannon>)>,
//     // mut mouse_pos: EventReader<CursorMoved>,
//     primary_window: Query<&Window, With<PrimaryWindow>>,
//     time: Res<Time>,
// ) {
//     if let Ok(mut window) = primary_window.get_single() {
//         let window_width = window.width() as f32;
//         let window_x_center = window_width / 2.0;
//         let max_offset = ((window_width / 2.0) * 0.75).min(160.0);

//         if let Some(cursor) = window.cursor_position() {
//             for mut transform in q_cannon.iter_mut() {
//                 let earth_transform = q_earth.single();

//                 let cursor_x_offset_from_center = cursor.x - window_x_center;
//                 let x_pos = cursor_x_offset_from_center.clamp(-max_offset, max_offset);
//                 let x_pos_rel = x_pos / max_offset;
//                 let angle = x_pos_rel * PI / 2.0 * 0.9;
//                 let radius = 6.0;
//                 let x = angle.sin() * radius;
//                 let y = angle.cos() * radius;

//                 let n = time.delta_seconds() * 16.0;

//                 let current_translation = transform.translation;
//                 let target_translation = Vec3::new(x, y, 0.0) + earth_transform.translation;
//                 let current_rotation = transform.rotation;
//                 //lookat
//                 let target_rotation = Quat::from_rotation_z(-angle);
//                 let new_translation = current_translation.lerp(target_translation, n);
//                 let new_rotation = current_rotation.lerp(target_rotation, n);
//                 transform.translation = new_translation;
//                 transform.rotation = new_rotation;
//             }
//         }
//     }
// }

fn rotate_crates(time: Res<Time>, mut q_crate: Query<&mut Transform, With<Crate>>) {
    for mut transform in q_crate.iter_mut() {
        transform.rotate(Quat::from_rotation_z(time.delta_seconds() * 2.0));
    }
}

fn spin_earth(time: Res<Time>, mut q_earth: Query<&mut Transform, With<Earth>>) {
    for mut transform in q_earth.iter_mut() {
        transform.rotation =
            Quat::from_rotation_x(-1.0) * Quat::from_rotation_y(time.elapsed_seconds() * 0.2);
    }
}

fn spin_crates(time: Res<Time>, mut q_crate: Query<&mut Transform, With<Crate>>) {
    for mut transform in q_crate.iter_mut() {
        transform.rotate(Quat::from_rotation_z(time.delta_seconds() * 1.0));
    }
}

fn spin_debris(
    time: Res<Time>,
    mut q_debris: Query<(Entity, &mut Transform), (With<Debris>, Without<PickedUp>)>,
) {
    for (entity, mut transform) in q_debris.iter_mut() {
        let id = entity.index();

        let x_spin_rate = 0.5 + (id % 10) as f32 * 0.1;
        let y_spin_rate = 0.5 + (id % 10) as f32 * 0.1;
        // let z_spin_rate = 1.0 + (id % 10) as f32 * 0.1;

        let e = time.elapsed_seconds() * 1.0;

        transform.rotation = Quat::from_euler(EulerRot::XYZ, e * x_spin_rate, e * y_spin_rate, 0.0);
    }
}

fn interact_play_button(
    mut q_button: Query<(&Interaction, &mut Style), (Changed<Interaction>, With<PlayButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some((interaction, mut style)) = q_button.iter_mut().next() {
        match interaction {
            Interaction::Pressed => {
                style.display = Display::None;
                next_state.set(GameState::ReadyToLaunch);
            }
            _ => {}
        };
    }
}

#[derive(Resource)]
struct Score(usize);

#[derive(Resource)]
struct Level(usize);

#[derive(Resource)]
struct LaunchPower(Stopwatch);

fn on_enter_menu(
    mut commands: Commands,
    music_controller: Query<&AudioSink, With<Music>>,
    mut q_score_text: Query<(Entity, &mut Style, &mut Text), With<ScoreText>>,
    level: Res<Level>,
) {
    // set music volume
    for sink in music_controller.iter() {
        sink.set_volume(MENU_MUSIC_VOLUME);
    }

    commands
        .spawn_text_button("Play", PRIMARY_COLOR_HUE)
        .insert(PlayButton);

    // update level text
    for (ent, mut style, mut text) in q_score_text.iter_mut() {
        // set visible
        style.display = Display::Flex;

        for section in text.sections.iter_mut() {
            section.value = format!("Level {}", level.0);
        }
    }
}

fn on_exit_menu(mut q_score_text: Query<(Entity, &mut Style, &mut Text), With<ScoreText>>) {
    for (ent, mut style, mut text) in q_score_text.iter_mut() {
        // set hidden
        style.display = Display::None;
    }
}

#[derive(Resource)]
struct GameTime(Stopwatch);

fn on_enter_playing(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut q_score_text: Query<&mut Style, With<ScoreText>>,
    music_controller: Query<&AudioSink, With<Music>>,
    q_earth: Query<Entity, With<Earth>>,
    // circle_mesh: Res<AssetHandle<Circle, Mesh>>,
    // circle_mat: Res<AssetHandle<Circle, ColorMaterial>>,
    debris_scene: Res<AssetHandle<Debris, Scene>>,
    level: Res<Level>,
    q_debris: Query<Entity, With<Debris>>,
) {
    // reset score
    // score.0 = 0;

    // // hide score text
    // for mut style in q_score_text.iter_mut() {
    //     style.display = Display::None;
    // }

    // start stopwatch
    commands.insert_resource(GameTime(Stopwatch::new()));

    // increase music volume
    for sink in music_controller.iter() {
        sink.set_volume(PLAYING_MUSIC_VOLUME);
    }

    // spawn one circle
    // commands.spawn((
    //     MaterialMesh2dBundle {
    //         mesh: circle_mesh.handle.clone().into(),
    //         material: circle_mat.handle.clone().into(),
    //         transform: Transform::from_translation(vec3(0.0, 0.0, 0.0)),
    //         ..Default::default()
    //     },
    //     Circle,
    //     PickableBundle::default(),
    //     RaycastPickTarget::default(),
    //     On::<Pointer<Down>>::run(on_click_circle),
    // ));

    // ensure Visibility::Visible on earth
    for earth_ent in q_earth.iter() {
        commands.entity(earth_ent).insert(Visibility::Visible);
    }

    // despawn all existing debris
    for debris_ent in q_debris.iter() {
        commands.entity(debris_ent).despawn_recursive();
    }

    // // spawn debris
    // for x in -3..=3 {
    //     for y in -3..=3 {
    //         commands
    //             .spawn((
    //                 Debris,
    //                 SceneBundle {
    //                     scene: asset_server.load("debris.glb#Scene0"),
    //                     transform: Transform::from_xyz(x as f32 * 4.0, y as f32 * 4.0, 0.0)
    //                         .with_scale(Vec3::splat(1.0))
    //                         .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0)),
    //                     ..default()
    //                 },
    //             ));
    //     }
    // }
    // spawn debris in a circle around sun
    let radius = 22.0 + level.0 as f32 * 1.0;
    let num_debris: i32 = 0 + level.0 as i32 * 1 + (level.0 as i32 - 2).max(0) * 2;
    for i in 0..num_debris {
        let mut angle = i as f32 / num_debris as f32 * PI * 2.0 + PI - 0.7 + level.0 as f32 + 3.9;
        angle *= 1.0 + level.0 as f32 * 0.38;
        let x = angle.sin() * radius + (i as f32 * 12.5 + 1.0).sin() * 5.0;
        let y = angle.cos() * radius + (i as f32 * 48.3 + 4.0).sin() * 5.0;
        commands.spawn((
            Debris,
            SceneBundle {
                scene: debris_scene.handle.clone(),
                transform: Transform::from_xyz(x, y + 15.0, 0.0)
                    .with_scale(Vec3::splat(2.0))
                    .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0 + x, 0.0 + y * 2.0, 0.0)),
                ..default()
            },
            // Velocity(vec2(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)),
        ));
    }
}

fn on_exit_playing(
    mut commands: Commands,
    mut q_score_text: Query<(&mut Style, &mut Text), With<ScoreText>>,
    score: Res<Score>,
) {
    // display score text
    for (mut style, mut text) in q_score_text.iter_mut() {
        style.display = Display::Flex;
        for section in text.sections.iter_mut() {
            section.value = format!("Score: {}", score.0);
        }
    }

    // remove sw
    commands.remove_resource::<GameTime>();
}

fn exit_on_esc(keyboard_input: ResMut<Input<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}

fn update_music_speed(
    // music_controller: Query<&AudioSink, With<Music>>,
    // sw: Option<Res<GameTime>>,
    time: Res<Time>,
    q_current_crate: Query<(&Transform, &Velocity), With<CurrentCrate>>,
    whining_controller: Query<&AudioSink, With<WhiningSound>>,
) {
    if let Ok(whining_sink) = whining_controller.get_single() {
        let current_volume = whining_sink.volume();
        let current_speed = whining_sink.speed();

        let mut target_volume = 0.0;
        let mut target_speed = 0.1;

        if let Ok((crate_transform, crate_velocity)) = q_current_crate.get_single() {
            let crate_pos = crate_transform.translation.xy();
            let crate_speed = crate_velocity.0.length();

            target_volume = 0.1 + (crate_speed * 0.25);
            target_speed = 1.0 + (crate_speed * 0.1);
        }

        let n = time.delta_seconds() * 6.0;

        let new_speed = current_speed * (1.0 - n) + target_speed * n;
        let new_volume = current_volume * (1.0 - n) + target_volume * n;

        whining_sink.set_speed(new_speed.clamp(0.1, 20.0));
        whining_sink.set_volume(new_volume.clamp(0.0, 1.0));
    }
}

#[derive(Component)]
struct PickedUp;

fn attach_debris_to_crate_collision(
    mut commands: Commands,
    mut q_crate: Query<(Entity, &Transform, &mut Mass), With<CurrentCrate>>,
    mut q_debris: Query<
        (Entity, &mut Transform),
        (With<Debris>, Without<PickedUp>, Without<CurrentCrate>),
    >,
) {
    for (crate_ent, crate_transform, mut crate_mass) in q_crate.iter_mut() {
        for (debris_ent, mut debris_transform) in q_debris.iter_mut() {
            let debris_pos = debris_transform.translation.xy();
            let crate_pos = crate_transform.translation.xy();

            let distance = debris_pos.distance(crate_pos);
            if distance < 3.0 {
                // attach debris to crate
                commands.entity(crate_ent).add_child(debris_ent);
                commands.entity(debris_ent).insert(PickedUp);

                let diff = debris_transform.translation - crate_transform.translation;

                // new debris pos = diff transformed by crate rotation
                let new_debris_pos = crate_transform.rotation.inverse() * diff;
                debris_transform.translation = new_debris_pos;

                // increase crate mass
                crate_mass.0 += 0.22;
            }
        }
    }
}

fn remove_crate_on_earth_collision(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut q_crate: Query<(Entity, &Transform), With<Crate>>,
    q_earth: Query<(Entity, &Transform), With<Earth>>,
    mut next_state: ResMut<NextState<GameState>>,
    explosion_mesh: Res<AssetHandle<Explosion, Mesh>>,
    explosion_mtl: Res<AssetHandle<Explosion, StandardMaterial>>,
) {
    for (crate_ent, crate_transform) in q_crate.iter_mut() {
        for (earth_ent, earth_transform) in q_earth.iter() {
            let earth_pos = earth_transform.translation.xy();
            let crate_pos = crate_transform.translation.xy();
            let distance = earth_pos.distance(crate_pos);
            if distance < 5.0 {
                // remove crate
                commands.entity(crate_ent).despawn_recursive();

                // reset score
                score.0 = 0;

                // play earth destroyed sound
                commands.spawn((
                    AudioBundle {
                        source: asset_server.load("earth_destroyed.ogg"),
                        settings: PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            volume: Volume::Relative(VolumeLevel::new(0.5)),
                            paused: false,
                            ..default()
                        },
                        ..default()
                    },
                    EarthDestroyedSound,
                ));

                // spawn explosion
                commands.spawn((
                    PbrBundle {
                        mesh: explosion_mesh.handle.clone().into(),
                        material: explosion_mtl.handle.clone().into(),
                        transform: Transform::from_xyz(earth_pos.x, earth_pos.y, 1.0)
                            .with_scale(Vec3::splat(7.0)),
                        ..default()
                    },
                    Explosion,
                ));
                for _ in 0..25 {
                    commands.spawn((
                        PbrBundle {
                            mesh: explosion_mesh.handle.clone().into(),
                            material: explosion_mtl.handle.clone().into(),
                            transform: Transform::from_xyz(earth_pos.x, earth_pos.y, 1.0)
                                .with_scale(Vec3::splat(rand::random::<f32>())),
                            ..default()
                        },
                        Explosion,
                        Velocity(vec2(
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                        )),
                    ));
                }

                // hide earth
                commands.entity(earth_ent).insert(Visibility::Hidden);

                // enter ready to launch state
                next_state.set(GameState::Menu);
            }
        }
    }
}

fn remove_crate_on_sun_collision(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut q_crate: Query<(Entity, &GlobalTransform), With<Crate>>,
    q_sun: Query<&Transform, With<Sun>>,
    mut next_state: ResMut<NextState<GameState>>,
    explosion_mesh: Res<AssetHandle<Explosion, Mesh>>,
    explosion_mtl: Res<AssetHandle<Explosion, StandardMaterial>>,
    q_floating_debris: Query<Entity, (With<Debris>, Without<PickedUp>)>,
    mut level: ResMut<Level>,
) {
    for (crate_ent, crate_global_transform) in q_crate.iter_mut() {
        for sun_transform in q_sun.iter() {
            let sun_pos = sun_transform.translation.xy();
            let crate_pos = crate_global_transform.translation().xy();
            let distance = sun_pos.distance(crate_pos);
            if distance < 13.0 {
                // remove crate
                commands.entity(crate_ent).despawn_recursive();

                // increase score
                score.0 += 1;

                // play earth destroyed sound
                commands.spawn((
                    AudioBundle {
                        source: asset_server.load("earth_destroyed.ogg"),
                        settings: PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            volume: Volume::Relative(VolumeLevel::new(0.4)),
                            paused: false,
                            ..default()
                        },
                        ..default()
                    },
                    EarthDestroyedSound,
                ));

                // spawn explosion
                commands.spawn((
                    PbrBundle {
                        mesh: explosion_mesh.handle.clone().into(),
                        material: explosion_mtl.handle.clone().into(),
                        transform: Transform::from_xyz(crate_pos.x, crate_pos.y, 1.0)
                            .with_scale(Vec3::splat(2.0)),
                        ..default()
                    },
                    Explosion,
                ));

                // if 0 debris, increase level
                let num_debris = q_floating_debris.iter().count();
                if num_debris == 0 {
                    level.0 += 1;

                    // enter menu state for next level
                    next_state.set(GameState::Menu);
                } else {
                    // enter ready to launch state
                    next_state.set(GameState::ReadyToLaunch);
                }
            }
        }
    }
}

fn while_playing(
    time: Res<Time>,
    mut commands: Commands,
    mut game_time: ResMut<GameTime>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    game_time.0.tick(time.delta());
}

fn update_camera_position(
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<CurrentCrate>)>,
    q_current_crate: Query<&Transform, With<CurrentCrate>>,
    time: Res<Time>,
) {
    let mut target_camera_xy = vec2(0.0, 0.0);

    if let Ok(current_crate_transform) = q_current_crate.get_single() {
        let crate_xy = current_crate_transform.translation.xy();
        target_camera_xy = crate_xy;
    }

    for mut camera_transform in q_camera.iter_mut() {
        let current_camera_xy = camera_transform.translation.xy();
        let new_camera_xy = current_camera_xy.lerp(target_camera_xy, time.delta_seconds() * 2.0);
        let new_camera_translation = vec3(new_camera_xy.x, new_camera_xy.y, 10.0);

        camera_transform.translation = new_camera_translation;
    }
}

fn always(time: Res<Time>, mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {}

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e9"]
pub struct SunMaterial {
    #[uniform(0)]
    color: Color,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Handle<Image>,
}
impl Material for SunMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sun.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e8"]
pub struct BackgroundMaterial {
    #[uniform(0)]
    color: Color,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Handle<Image>,
}
impl Material for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/background.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
