use bevy::math::bounding::{Aabb2d, BoundingCircle, IntersectsVolume};
use bevy::prelude::*;
use bevy::sprite::{Wireframe2dConfig, Wireframe2dPlugin};
use rand::prelude::*;

#[derive(Resource)]
struct EnemySpawnTimer(Timer);

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Velocity(f32);

#[derive(Component)]
struct Direction(Vec3);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Collided(bool);

#[derive(Component)]
struct Range(f32);

#[derive(Component)]
struct Projectile;

#[derive(Component)]
struct Target(Option<Entity>);

#[derive(Component)]
struct FireRate(f32);

#[derive(Component)]
struct Cooldown(Timer);

fn setup_tower(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    let shape = meshes.add(Rectangle::new(40., 40.));
    let color = Color::hsl(360., 0.95, 0.7);

    commands.spawn((
        Mesh2d(shape),
        MeshMaterial2d(materials.add(color)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Range(200.0),
        FireRate(1.0),
        Cooldown(Timer::from_seconds(0.2, TimerMode::Repeating)),
        Player,
        Target(None),
    ));
}

fn spawn_enemy(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<&Transform, With<Player>>,
    window: Single<&Window>,
) {
    let mut rng = rand::thread_rng();
    if spawn_timer.0.tick(time.delta()).just_finished() {
        for player_transform in &query {
            let shape = meshes.add(Rectangle::new(10.0, 10.0));
            let color = Color::hsl(360., 0.95, 0.7);
            let dir =
                Vec2::new(rng.gen_range(-100.0..100.0), rng.gen_range(-100.0..100.0)).normalize();
            let enemy_center = Vec2::new(window.width() / 2., window.height() / 2.)
                + Vec2::new(window.width(), window.height()) * dir;
            let enemy_transform = Transform::from_xyz(enemy_center.x, enemy_center.y, 0.0);

            commands.spawn((
                Mesh2d(shape),
                MeshMaterial2d(materials.add(color)),
                enemy_transform,
                Enemy,
                Velocity(100.0),
                Direction(player_transform.translation - enemy_transform.translation),
                Collided(false),
            ));
        }
    }
}

fn update_enemy_position(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Direction, &Velocity), With<Enemy>>,
) {
    for (mut transform, direction, velocity) in &mut query {
        transform.translation += direction.0.normalize() * velocity.0 * time.delta_secs();
    }
}

fn check_enemy_player_collision(
    player_transform: Single<&Transform, With<Player>>,
    mut query: Query<(&Transform, &mut Collided), With<Enemy>>,
) {
    for (enemy_transform, mut collided) in &mut query {
        let enemy_bounding = Aabb2d::new(
            enemy_transform.translation.truncate(),
            enemy_transform.scale.truncate() / 2.,
        );

        let player_bounding = Aabb2d::new(
            player_transform.translation.truncate(),
            player_transform.scale.truncate() / 2.,
        );

        if enemy_bounding.intersects(&player_bounding) {
            collided.0 = true;
        }
    }
}

fn despawn_collided_enemies(
    mut commands: Commands,
    mut query: Query<(Entity, &Collided), With<Enemy>>,
) {
    for (entity, collided) in &mut query {
        if collided.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn tower_choose_target(
    query: Query<(Entity, &Transform), With<Enemy>>,
    player: Single<(&Transform, &mut Target), With<Player>>,
) {
    let (player_transform, mut target) = player.into_inner();

    let mut closest_enemy: Option<Entity> = None;
    let mut distance_to_player = f32::MAX;
    for (entity, enemy_transform) in &query {
        let curr_distance_to_player = enemy_transform
            .translation
            .distance(player_transform.translation);
        if curr_distance_to_player < distance_to_player {
            closest_enemy = Some(entity);
            distance_to_player = curr_distance_to_player;
        }
    }

    target.0 = closest_enemy;
}

fn tower_shoot_target(
    time: Res<Time>,
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Enemy>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    player: Single<(&Range, &mut Cooldown, &Transform, &mut Target), With<Player>>,
) {
    let (player_range, mut cooldown, player_transform, target) = player.into_inner();

    cooldown.0.tick(time.delta());
    if let Some(enemy) = target.0 {
        let color = Color::hsl(360., 0.95, 0.7);

        if let Ok((_, enemy_transform)) = query.get(enemy) {
            let distance_to_player = enemy_transform
                .translation
                .distance(player_transform.translation);

            if distance_to_player < player_range.0 {
                if cooldown.0.just_finished() {
                    commands.spawn((
                        Mesh2d(meshes.add(Circle::new(5.0))),
                        MeshMaterial2d(materials.add(color)),
                        player_transform.clone(),
                        Velocity(100.0),
                        Projectile,
                        Target(Some(enemy)),
                        Direction(enemy_transform.translation - player_transform.translation),
                    ));
                }
            }
        }
    }
}

fn update_projectiles_position(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Direction, &Velocity), With<Projectile>>,
) {
    for (mut transform, direction, velocity) in &mut query {
        transform.translation += direction.0.normalize() * velocity.0 * time.delta_secs();
    }
}

fn check_projectile_collision(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &Direction, &Velocity, &Target), With<Projectile>>,
    enemies: Query<&Transform, With<Enemy>>,
) {
    for (projectile_entity, transform, _direction, _velocity, &Target(maybe_enemy_entity)) in
        &mut query
    {
        let enemy_entity =
            maybe_enemy_entity.expect("Projectiles are alawys expected to have a target?");

        if let Ok(enemy_transform) = enemies.get(enemy_entity) {
            let bounding_circle = BoundingCircle::new(transform.translation.truncate(), 5.0 / 2.);
            let bounding_box = Aabb2d::new(
                enemy_transform.translation.truncate(),
                enemy_transform.scale.truncate() / 2.,
            );

            if bounding_circle.intersects(&bounding_box) {
                commands.entity(enemy_entity).despawn();
                commands.entity(projectile_entity).despawn();
            }
        } else {
            commands.entity(projectile_entity).despawn();
        }
    }
}

pub struct HelloPlugin;
impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemySpawnTimer(Timer::from_seconds(
            2.0,
            TimerMode::Repeating,
        )));
        app.add_systems(Startup, setup_tower);
        app.add_systems(
            Update,
            (
                spawn_enemy,
                update_enemy_position,
                (tower_choose_target, tower_shoot_target).chain(),
                (update_projectiles_position, check_projectile_collision).chain(),
                (check_enemy_player_collision, despawn_collided_enemies).chain(),
            ),
        );
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Wireframe2dPlugin)
        .add_plugins(HelloPlugin)
        .run();
}
