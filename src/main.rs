use bevy::{prelude::*, time::FixedTimestep};
use rand::prelude::*;

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 500.0,
            height: 500.0,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_startup_system(setup)
        .add_startup_system(spawn_snake)
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(size_scaling)
                .with_system(position_scaling),
        )
        .add_system(snake_movement_input.before(snake_movement))
        .insert_resource(LastTailPosition::default())
        .add_event::<GrowthEvent>()
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.150))
                .with_system(snake_movement)
                .with_system(snake_eating.after(snake_movement))
                .with_system(snake_growth.after(snake_movement))
        )
        .insert_resource(SnakeSegments::default())
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(1.0))
                .with_system(food_spawner),
        )
        .add_event::<GameOverEvent>()
        .add_system(game_over.after(snake_movement))
        .run();
}

fn setup(mut commands: Commands) {
    // Setup the 2D camera system.
    commands.spawn_bundle(Camera2dBundle::default());
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    fn square(size: f32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }
}

fn size_scaling(windows: Res<Windows>, mut query: Query<(&Size, &mut Transform)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut transform) in &mut query {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn position_scaling(windows: Res<Windows>, mut query: Query<(&Position, &mut Transform)>) {
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in &mut query {
        transform.translation = Vec3::new(
            convert_position(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert_position(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn convert_position(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
    let tile_size = bound_window / bound_game;
    pos / bound_game * bound_window - (bound_window / 2.0) + (tile_size / 2.0)
}

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        commands
            // Initialize the head of the snake using its corresponding component
            // and a bundle containing a sprite.
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                transform: Transform {
                    scale: Vec3::new(10.0, 10.0, 0.0),
                    ..default()
                },
                ..default()
            })
            .insert(SnakeHead {
                direction: Direction::Up,
            })
            .insert(SnakeSegment)
            .insert(Position { x: 3, y: 3 })
            .insert(Size::square(0.8))
            .id(),
        spawn_segment(commands, Position { x: 3, y: 2 }),
    ]);
}

fn snake_movement(
    mut heads: Query<(Entity, &SnakeHead)>,
    segments: ResMut<SnakeSegments>,
    mut positions: Query<&mut Position>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_writer: EventWriter<GameOverEvent>,
) {
    if let Some((head_entity, head)) = heads.iter_mut().next() {
        let segment_positions = segments
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<_>>();

        let mut head_pos = positions.get_mut(head_entity).unwrap();

        match &head.direction {
            Direction::Left => head_pos.x -= 1,
            Direction::Right => head_pos.x += 1,
            Direction::Down => head_pos.y -= 1,
            Direction::Up => head_pos.y += 1,
        }

        if head_pos.x < 0 {
            head_pos.x = ARENA_WIDTH as i32 - 1;
        } else if head_pos.y < 0 {
            head_pos.y = ARENA_HEIGHT as i32 - 1;
        } else if head_pos.x as u32 >= ARENA_WIDTH {
            head_pos.x = 0;
        } else if head_pos.y as u32 >= ARENA_HEIGHT {
            head_pos.y = 0;
        }

        if segment_positions.contains(&head_pos) {
            game_over_writer.send(GameOverEvent);
        }

        segment_positions
            .iter()
            .zip(segments.iter().skip(1))
            .for_each(|(previous_seg_pos, current_segment)| {
                *positions.get_mut(*current_segment).unwrap() = *previous_seg_pos;
            });

        *last_tail_position = LastTailPosition(segment_positions.iter().last().copied())
    }
}

fn snake_movement_input(keyboard: Res<Input<KeyCode>>, mut heads: Query<&mut SnakeHead>) {
    if let Some(mut head) = heads.iter_mut().next() {
        // Using else ifs makes the movement mutually exclusive.
        let keyboard_direction = if keyboard.pressed(KeyCode::Left) {
            Direction::Left
        } else if keyboard.pressed(KeyCode::Right) {
            Direction::Right
        } else if keyboard.pressed(KeyCode::Down) {
            Direction::Down
        } else if keyboard.pressed(KeyCode::Up) {
            Direction::Up
        } else {
            head.direction
        };

        // A snake head can't just turn around!
        if keyboard_direction != head.direction.opposite() {
            head.direction = keyboard_direction;
        }
    }
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Deref, DerefMut)]
struct SnakeSegments(Vec<Entity>);

fn spawn_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: SNAKE_SEGMENT_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(0.65))
        .id()
}

#[derive(Component)]
struct Food;

fn food_spawner(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Food)
        .insert(Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        })
        .insert(Size::square(0.8));
}

struct GrowthEvent;

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    if let Some(head_pos) = head_positions.iter().next() {
        for (food_entity, food_pos) in &food_positions {
            if food_pos == head_pos {
                commands.entity(food_entity).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

#[derive(Default)]
struct LastTailPosition(Option<Position>);

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    // We don't actually care about the value, since the event itself is an
    // empty struct. We care wheter it exists.
    if growth_reader.iter().next().is_some() {
        segments.push(spawn_segment(commands, last_tail_position.0.unwrap()));
    }
}

struct GameOverEvent;

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    segments: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segment_entities: Query<Entity, With<SnakeSegment>>,
) {
    if reader.iter().next().is_some() {
        for entity in food.iter().chain(segment_entities.iter()) {
            commands.entity(entity).despawn();
        }

        spawn_snake(commands, segments);
    }
}
