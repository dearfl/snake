use bevy::color::palettes::{
    css::{GREEN, ORANGE},
    tailwind::{GRAY_300, ORANGE_100},
};
pub use bevy::prelude::*;

const X: i32 = 16;
const Y: i32 = 16;
const UNIT: f32 = 48.0;
const GAP: f32 = 4.0;
const PADDING: f32 = 24.0;

pub const SCREEN_WIDTH: f32 = X as f32 * UNIT + (X - 1) as f32 * GAP + PADDING * 2.0;
pub const SCREEN_HEIGHT: f32 = Y as f32 * UNIT + (Y - 1) as f32 * GAP + PADDING * 2.0;

#[derive(Clone, Debug, Resource)]
pub struct Materials {
    wall: Handle<ColorMaterial>,
    food: Handle<ColorMaterial>,
    head: Handle<ColorMaterial>,
    body: Handle<ColorMaterial>,
    mesh: Handle<Mesh>,
}

impl FromWorld for Materials {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        let wall = mats.add(Color::from(GRAY_300));
        let food = mats.add(Color::from(GREEN));
        let head = mats.add(Color::from(ORANGE));
        let body = mats.add(Color::from(ORANGE_100));
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let mesh = meshes.add(Rectangle::from_length(UNIT));
        Self {
            wall,
            food,
            head,
            body,
            mesh,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Left,
    Down,
    Right,
}

impl Direction {
    pub fn opposite(self, rhs: Self) -> bool {
        matches!(
            (self, rhs),
            (Direction::Up, Direction::Down)
                | (Direction::Left, Direction::Right)
                | (Direction::Down, Direction::Up)
                | (Direction::Right, Direction::Left)
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Wall;

#[derive(Clone, Copy, Debug)]
pub struct Food;

#[derive(Clone, Copy, Debug)]
pub struct Head(Direction);

#[derive(Clone, Copy, Debug)]
pub struct Body;

pub trait ZIndex {
    fn zindex(&self) -> f32;
}

impl ZIndex for Head {
    fn zindex(&self) -> f32 {
        1.0
    }
}

impl ZIndex for Body {
    fn zindex(&self) -> f32 {
        1.0
    }
}

impl ZIndex for Wall {
    fn zindex(&self) -> f32 {
        1.0
    }
}

impl ZIndex for Food {
    fn zindex(&self) -> f32 {
        0.0
    }
}

#[derive(Debug, Component)]
#[require(Transform, Visibility)]
pub struct Cell<T> {
    col: i32,
    row: i32,
    value: T,
}

impl<T> Cell<T> {
    pub fn new(col: i32, row: i32, t: T) -> Self {
        Self { col, row, value: t }
    }

    pub fn collision<S>(&self, rhs: &Cell<S>) -> bool {
        self.col == rhs.col && self.row == rhs.row
    }
}

impl<T: ZIndex> Cell<T> {
    pub fn transform(&self) -> Transform {
        let x = (self.col - X / 2) as f32 * (UNIT + GAP) + UNIT / 2.0;
        let y = (self.row - Y / 2) as f32 * (UNIT + GAP) + UNIT / 2.0;
        Transform::from_xyz(x, y, self.value.zindex())
    }
}

impl Cell<Head> {
    pub fn move_forward(&mut self) {
        match self.value.0 {
            Direction::Up => {
                self.row += 1;
            }
            Direction::Left => {
                self.col -= 1;
            }
            Direction::Down => {
                self.row -= 1;
            }
            Direction::Right => {
                self.col += 1;
            }
        }
    }

    pub fn change_direction(&mut self, direction: Direction) {
        if !self.value.0.opposite(direction) {
            self.value.0 = direction;
        }
    }
}

#[derive(Debug, Component)]
#[relationship(relationship_target = FollowedBy)]
pub struct Following(Entity);

#[derive(Debug, Component)]
#[relationship_target(relationship = Following)]
pub struct FollowedBy(Entity);

#[derive(Debug, Resource, Deref, DerefMut, Default)]
pub struct NextDirection(Option<Direction>);

#[derive(Debug, Resource, Deref, DerefMut, Default)]
pub struct Growth(bool);

impl Growth {
    pub fn replace(&mut self, value: bool) -> bool {
        let ret = self.0;
        self.0 = value;
        ret
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct UpdateTimer(Timer);

impl Default for UpdateTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

fn setup(mut command: Commands, materials: Res<Materials>) {
    command.spawn((
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            hdr: true,
            ..Default::default()
        },
        Msaa::Off,
    ));

    (0..(X * Y))
        .map(|idx| (idx % X, idx / X))
        .filter(|&(col, row)| col == 0 || col == (X - 1) || row == 0 || row == (Y - 1))
        .for_each(|(col, row)| {
            let wall = Cell::new(col, row, Wall);
            let transform = wall.transform();
            command.spawn((
                #[cfg(feature = "debug")]
                Name::new("Wall"),
                wall,
                transform,
                Visibility::Visible,
                Mesh2d(materials.mesh.clone()),
                MeshMaterial2d(materials.wall.clone()),
            ));
        });

    let row = Y / 2;
    let col = X / 4;
    let head = Cell::new(col, row, Head(Direction::Right));
    let head_transform = head.transform();
    let body = Cell::new(col - 1, row, Body);
    let body_transform = body.transform();
    // This is actually body following head
    command
        .spawn((
            #[cfg(feature = "debug")]
            Name::new("Head"),
            head,
            head_transform,
            Visibility::Visible,
            Mesh2d(materials.mesh.clone()),
            MeshMaterial2d(materials.head.clone()),
        ))
        .with_related::<Following>((
            #[cfg(feature = "debug")]
            Name::new("Body"),
            body,
            body_transform,
            Visibility::Visible,
            Mesh2d(materials.mesh.clone()),
            MeshMaterial2d(materials.body.clone()),
        ));
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn move_snake(
    mut command: Commands,
    time: Res<Time>,
    materials: Res<Materials>,
    mut update_timer: ResMut<UpdateTimer>,
    mut direction_change: ResMut<NextDirection>,
    mut growth: ResMut<Growth>,
    query_head: Single<(&mut Cell<Head>, &FollowedBy, &mut Transform), Without<Cell<Body>>>,
    // the last body have no FollowedBy component
    mut query_body: Query<
        (Entity, &mut Cell<Body>, Option<&FollowedBy>, &mut Transform),
        Without<Cell<Head>>,
    >,
) {
    if !update_timer.tick(time.delta()).just_finished() {
        return;
    }
    let (mut head, next, mut transform) = query_head.into_inner();
    let mut row = head.row;
    let mut col = head.col;
    let mut entity = Some(next.0);
    if let Some(direction) = direction_change.take() {
        head.change_direction(direction);
    }
    head.move_forward();
    *transform = head.transform();
    let mut last_body_part = next.0;
    while let Some((entity_body, mut body, next, mut transform)) =
        entity.and_then(|entity| query_body.get_mut(entity).ok())
    {
        let (c, r) = (body.col, body.row);
        body.col = col;
        body.row = row;
        *transform = body.transform();
        col = c;
        row = r;
        entity = next.map(|f| f.0);
        last_body_part = entity_body;
    }

    if growth.replace(false) {
        let body = Cell::new(col, row, Body);
        let body_transform = body.transform();
        command.entity(last_body_part).with_related::<Following>((
            #[cfg(feature = "debug")]
            Name::new("Body"),
            body,
            body_transform,
            Visibility::Visible,
            Mesh2d(materials.mesh.clone()),
            MeshMaterial2d(materials.body.clone()),
        ));
    }
}

fn check_collision(
    query_head: Single<&Cell<Head>>,
    query_body: Query<&Cell<Body>>,
    query_wall: Query<&Cell<Wall>>,
    mut exit: EventWriter<AppExit>,
) {
    let head = query_head.into_inner();
    let colli_body = query_body.iter().any(|body| head.collision(body));
    let colli_wall = query_wall.iter().any(|wall| head.collision(wall));
    if colli_body || colli_wall {
        exit.write(AppExit::Success);
    }
}

fn keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut direction_change: ResMut<NextDirection>,
) {
    let direction = if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        Direction::Left
    } else if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        Direction::Up
    } else if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        Direction::Down
    } else if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        Direction::Right
    } else {
        return;
    };
    direction_change.replace(direction);
}

fn consume_food(
    mut command: Commands,
    food: Single<(Entity, &Cell<Food>)>,
    head: Single<&Cell<Head>>,
    mut growth: ResMut<Growth>,
) {
    let (entity, food) = food.into_inner();
    let head = head.into_inner();
    if head.collision(food) {
        command.entity(entity).despawn();
        growth.replace(true);
    }
}

fn create_food_if_not_exist(
    mut command: Commands,
    materials: Res<Materials>,
    food: Option<Single<&Cell<Food>>>,
    head: Single<&Cell<Head>>,
    body: Query<&Cell<Body>>,
) {
    if food.is_some() {
        return;
    }
    let (col, row) = loop {
        let col = rand::random_range(1..(X - 1));
        let row = rand::random_range(1..(Y - 1));
        let colli_head = col == head.col && row == head.row;
        let colli_body = body.iter().any(|cell| cell.col == col && cell.row == row);
        if !(colli_head || colli_body) {
            break (col, row);
        }
    };
    let food = Cell::new(col, row, Food);
    let transform = food.transform();
    command.spawn((
        #[cfg(feature = "debug")]
        Name::new("Food"),
        food,
        transform,
        Visibility::Visible,
        Mesh2d(materials.mesh.clone()),
        MeshMaterial2d(materials.food.clone()),
    ));
}

#[derive(Clone, Copy, Debug)]
pub struct Snake;

impl Plugin for Snake {
    fn build(&self, app: &mut App) {
        app.init_resource::<Materials>()
            .init_resource::<UpdateTimer>()
            .init_resource::<NextDirection>()
            .init_resource::<Growth>()
            .add_systems(Startup, setup)
            .add_systems(
                FixedUpdate,
                (
                    create_food_if_not_exist,
                    move_snake,
                    check_collision,
                    keyboard_input,
                    consume_food,
                ),
            );
    }
}
