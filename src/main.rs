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

mod button;
// mod mute;
mod utils;

static PRIMARY_COLOR_HUE: f32 = 0.59;
static MENU_MUSIC_VOLUME: f32 = 0.36;
static PLAYING_MUSIC_VOLUME: f32 = 0.66;

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
    .insert_resource(Level(1))
    .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
    .insert_resource(LaunchPower(Stopwatch::new()))
    .insert_resource(PrimaryColorHue(PRIMARY_COLOR_HUE))
    .add_plugins(MaterialPlugin::<SunMaterial>::default())
    .add_plugins(MaterialPlugin::<BackgroundMaterial>::default())
    // .add_plugins(MuteButtonPlugin)
    .add_state::<GameState>()
    .add_systems(Startup, setup)
    .add_systems(OnEnter(GameState::MainMenu), on_enter_menu)
    .add_systems(OnEnter(GameState::Launched), on_enter_launched)
    .add_systems(OnEnter(GameState::ChargingLaunch), on_enter_charging)
    .add_systems(
        OnExit(GameState::MainMenu),
        (on_exit_menu, on_enter_playing),
    )
    // .add_systems(OnEnter(GameState::ReadyToLaunch), on_enter_playing)
    // .add_systems(OnExit(GameState::ReadyToLaunch), on_exit_playing)
    .add_systems(FixedUpdate, (apply_gravity))
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
        ),
    )
    .add_systems(
        Update,
        (start_launching).run_if(in_state(GameState::ReadyToLaunch)),
    )
    .add_systems(
        Update,
        (update_launch_power, launch).run_if(in_state(GameState::ChargingLaunch)),
    )
    .add_systems(
        Update,
        (interact_play_button,).run_if(in_state(GameState::MainMenu)),
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
    MainMenu,
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut sun_materials: ResMut<Assets<SunMaterial>>,
    mut bg_materials: ResMut<Assets<BackgroundMaterial>>,
    asset_server: Res<AssetServer>,
) {
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

    // charge sound
    commands.spawn((
        AudioBundle {
            source: asset_server.load("charge.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Once,
                volume: Volume::Relative(VolumeLevel::new(0.4)),
                paused: true,
                ..default()
            },
            ..default()
        },
        ChargeSound,
    ));

    // fire sound
    commands.spawn((
        AudioBundle {
            source: asset_server.load("fired.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Once,
                volume: Volume::Relative(VolumeLevel::new(0.4)),
                paused: true,
                ..default()
            },
            ..default()
        },
        FireSound,
    ));

    // earth destroyed sound
    commands.spawn((
        AudioBundle {
            source: asset_server.load("earth_destroyed.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Once,
                volume: Volume::Relative(VolumeLevel::new(0.4)),
                paused: true,
                ..default()
            },
            ..default()
        },
        EarthDestroyedSound,
    ));

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

    // spawn score text
    commands.spawn((
        ScoreText,
        TextBundle::from_section(
            format!("Score: 0"),
            TextStyle {
                font_size: 64.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            margin: UiRect::new(Val::Auto, Val::Auto, Val::Vh(20.0), Val::Auto),
            display: Display::None,
            ..default()
        }),
    ));

    // AssetHandle example
    // commands.insert_resource(AssetHandle::<Circle, ColorMaterial>::new(
    //     materials.add(Color::hsl((PRIMARY_COLOR_HUE - 0.5) * 360.0, 0.7, 0.8).into()),
    // ));

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

    // spawn debris in a circle
    let radius = 25.0;
    let num_debris = 8;
    for i in 0..num_debris {
        let angle = i as f32 / num_debris as f32 * PI * 2.0;
        let x = angle.sin() * radius;
        let y = angle.cos() * radius;
        commands
            .spawn((
                Debris,
                SceneBundle {
                    scene: asset_server.load("debris.glb#Scene0"),
                    transform: Transform::from_xyz(x, y + 15.0, 0.0)
                        .with_scale(Vec3::splat(1.0))
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0 + x, 0.0 + y * 2.0, 1.0)),
                    ..default()
                },
            ));
    }

    // spawn cannon + crate
    commands
        .spawn((
            Cannon,
            SceneBundle {
                scene: asset_server.load("launcher.glb#Scene0"),
                transform: Transform::from_xyz(0.0, 0.0, 0.0)
                    // .with_scale(Vec3::splat(5.0))
                    .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0))
                    ,
                ..default()
            },
            // SpatialBundle {
            //     transform: Transform::from_translation(vec3(0.0, 0.0, 0.0)),
            //     ..default()
            // },
        ))
        .with_children(|parent| {
            parent.spawn((
                SceneBundle {
                    scene: asset_server.load("crate.glb#Scene0"),
                    transform: Transform::from_xyz(0.0, 3.0, 0.0)
                        .with_scale(Vec3::splat(1.0))
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 0.0, 1.0)),
                    ..default()
                },
                Crate,
                Mass(1.0),
                CurrentCrate,
            ));
        });
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

fn on_enter_charging(
    charge_sound_controller: Query<&AudioSink, With<ChargeSound>>
) {
    for sink in charge_sound_controller.iter() {
        sink.play();
    }
}

fn on_enter_launched(
    mut commands: Commands,
    launch_power: Res<LaunchPower>,
    mut current_crate: Query<(Entity, &mut Transform, &GlobalTransform), (With<CurrentCrate>, Without<Cannon>, Without<Earth>)>,
    cannon: Query<&Transform, (With<Cannon>, Without<Earth>)>,
    earth: Query<&Transform, (With<Earth>, Without<Cannon>)>,
    charge_sound_controller: Query<&AudioSink, With<ChargeSound>>,
    fire_sound_controller: Query<&AudioSink, With<FireSound>>,
) {
    for sink in charge_sound_controller.iter() {
        sink.pause();
    }

    for sink in fire_sound_controller.iter() {
        sink.play();
    }


    let (crate_ent, mut crate_transform, crate_global_transform) = current_crate.single_mut();
    let cannon_transform = cannon.single().clone();
    let earth_transform = earth.single().clone();
    let translation_diff = cannon_transform.translation - earth_transform.translation;
    let diff_normal = translation_diff.normalize().xy();

    // add Velocity to current crate
    let power = launch_power.0.elapsed_secs() * 1.0;
    commands
        .entity(crate_ent)
        .insert(Velocity(diff_normal * power));

    // add cannon translation to crate
    crate_transform.translation = crate_global_transform.translation();
    crate_transform.rotation = cannon_transform.rotation * crate_transform.rotation;

    // move current_crate from parent to root
    commands.entity(crate_ent).remove::<Parent>();

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
    }
}

fn apply_velocity(mut q_crate: Query<(&mut Transform, &Velocity), With<Crate>>, time: Res<Time>) {
    for (mut transform, velocity) in q_crate.iter_mut() {
        transform.translation +=
            Vec3::new(velocity.0.x, velocity.0.y, 0.0) * time.delta_seconds() * 50.0;
    }
}

// based on cursor position, move cannon in an arc around earth
fn update_cannon_transform(
    mut q_cannon: Query<&mut Transform, With<Cannon>>,
    q_earth: Query<&Transform, (With<Earth>, Without<Cannon>)>,
    // mut mouse_pos: EventReader<CursorMoved>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    if let Ok(mut window) = primary_window.get_single() {
        let window_width = window.width() as f32;
        let window_x_center = window_width / 2.0;
        let max_offset = ((window_width / 2.0) * 0.75).min(160.0);

        if let Some(cursor) = window.cursor_position() {
            for mut transform in q_cannon.iter_mut() {
                let earth_transform = q_earth.single();

                let cursor_x_offset_from_center = cursor.x - window_x_center;
                let x_pos = cursor_x_offset_from_center.clamp(-max_offset, max_offset);
                let x_pos_rel = x_pos / max_offset;
                let angle = x_pos_rel * PI / 2.0 * 0.8;
                let radius = 6.0;
                let x = angle.sin() * radius;
                let y = angle.cos() * radius;

                let n = time.delta_seconds() * 16.0;

                let current_translation = transform.translation;
                let target_translation = Vec3::new(x, y, 0.0) + earth_transform.translation;
                let current_rotation = transform.rotation;
                //lookat
                let target_rotation = Quat::from_rotation_z(-angle);
                let new_translation = current_translation.lerp(target_translation, n);
                let new_rotation = current_rotation.lerp(target_rotation, n);
                transform.translation = new_translation;
                transform.rotation = new_rotation;
            }
        }
    }
}

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

fn spin_debris(time: Res<Time>, mut q_debris: Query<(Entity, &mut Transform), With<Debris>>) {
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

fn on_enter_menu(mut commands: Commands, music_controller: Query<&AudioSink, With<Music>>) {
    // set music volume
    for sink in music_controller.iter() {
        sink.set_volume(MENU_MUSIC_VOLUME);
    }

    commands
        .spawn_text_button("Play", PRIMARY_COLOR_HUE)
        .insert(PlayButton);
}

fn on_exit_menu() {}

#[derive(Resource)]
struct GameTime(Stopwatch);

fn on_enter_playing(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut q_score_text: Query<&mut Style, With<ScoreText>>,
    music_controller: Query<&AudioSink, With<Music>>,
    // circle_mesh: Res<AssetHandle<Circle, Mesh>>,
    // circle_mat: Res<AssetHandle<Circle, ColorMaterial>>,
) {
    // reset score
    score.0 = 0;

    // hide score text
    for mut style in q_score_text.iter_mut() {
        style.display = Display::None;
    }

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
    music_controller: Query<&AudioSink, With<Music>>,
    sw: Option<Res<GameTime>>,
    time: Res<Time>,
) {
    let target_speed = if let Some(sw) = sw {
        1.0 + sw.0.elapsed_secs() * 0.015
    } else {
        1.0
    };

    for sink in music_controller.iter() {
        let current_speed = sink.speed();
        let n = time.delta_seconds() * 8.0;
        let new_speed = current_speed * (1.0 - n) + target_speed * n;
        sink.set_speed(new_speed.clamp(0.0, 5.0));
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
