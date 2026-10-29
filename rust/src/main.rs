#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
use macroquad::prelude::*;

const W: f32 = 960.0;
const H: f32 = 600.0;
const NEAR: f32 = 1.0;
const PLAYER_SPEED: f32 = 7.0;
const FIRE_INTERVAL: f32 = 0.18;
const REACTOR_DISTANCE: f32 = 560.0;
const BOLT_SPEED: f32 = 105.0;
const LEVEL_SPEED: [f32; 3] = [24.0, 31.0, 39.0];
const OBSTACLE_DELAY: [(f32, f32); 3] = [(5.0, 6.2), (3.4, 4.4), (2.1, 2.9)];

fn course_center(distance: f32) -> f32 {
    4.0 * (distance / 85.0).sin() + 1.7 * (distance / 33.0).sin()
}

fn course_height(distance: f32) -> f32 {
    2.6 * (distance / 92.0).sin() + 1.1 * (distance / 34.0).sin()
}

const BG: Color = Color::new(0.008, 0.031, 0.047, 1.0);
const GREEN: Color = Color::new(0.31, 0.93, 0.66, 1.0);
const DIM: Color = Color::new(0.12, 0.43, 0.36, 1.0);
const CYAN: Color = Color::new(0.29, 0.85, 0.95, 1.0);
const RED: Color = Color::new(1.0, 0.35, 0.36, 1.0);
const YELLOW: Color = Color::new(1.0, 0.82, 0.39, 1.0);
const WHITE: Color = Color::new(0.82, 0.96, 0.90, 1.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Target,
    Fighter,
    Reactor,
    Obstacle,
}

#[derive(Clone, Copy, Debug)]
struct Actor {
    kind: Kind,
    x: f32,
    y: f32,
    z: f32,
    shot_timer: f32,
}

#[derive(Clone, Copy, Debug)]
struct Bolt {
    x: f32,
    y: f32,
    z: f32,
    aim_x: f32,
    aim_y: f32,
    start_z: f32,
}

#[derive(Clone, Copy)]
struct Burst {
    screen: Vec2,
    age: f32,
    color: Color,
}

#[derive(Clone, Copy, Default)]
struct Controls {
    x: f32,
    y: f32,
    fire: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EndState {
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sfx {
    Laser,
    Hit,
    Damage,
    Warning,
    Victory,
    Defeat,
}

struct Sounds {
    laser: Option<Sound>,
    hit: Option<Sound>,
    damage: Option<Sound>,
    warning: Option<Sound>,
    victory: Option<Sound>,
    defeat: Option<Sound>,
}

impl Sounds {
    async fn load() -> Self {
        Self {
            laser: load_sound_from_bytes(include_bytes!("../../assets/laser.wav"))
                .await
                .ok(),
            hit: load_sound_from_bytes(include_bytes!("../../assets/hit.wav"))
                .await
                .ok(),
            damage: load_sound_from_bytes(include_bytes!("../../assets/damage.wav"))
                .await
                .ok(),
            warning: load_sound_from_bytes(include_bytes!("../../assets/warning.wav"))
                .await
                .ok(),
            victory: load_sound_from_bytes(include_bytes!("../../assets/victory.wav"))
                .await
                .ok(),
            defeat: load_sound_from_bytes(include_bytes!("../../assets/defeat.wav"))
                .await
                .ok(),
        }
    }

    fn play(&self, cue: Sfx) {
        let sound = match cue {
            Sfx::Laser => &self.laser,
            Sfx::Hit => &self.hit,
            Sfx::Damage => &self.damage,
            Sfx::Warning => &self.warning,
            Sfx::Victory => &self.victory,
            Sfx::Defeat => &self.defeat,
        };
        if let Some(sound) = sound {
            play_sound_once(sound);
        }
    }
}

struct Game {
    level: usize,
    speed: f32,
    player_x: f32,
    player_y: f32,
    distance: f32,
    score: u32,
    hull: i32,
    actors: Vec<Actor>,
    bolts: Vec<Bolt>,
    bursts: Vec<Burst>,
    spawn_timer: f32,
    obstacle_timer: f32,
    fire_timer: f32,
    flash_timer: f32,
    hit_flash: f32,
    feedback_timer: f32,
    feedback: String,
    feedback_color: Color,
    threat_timer: f32,
    wall_timer: f32,
    reactor_spawned: bool,
    end_state: Option<EndState>,
    rng: u64,
    muted: bool,
    audio_events: Vec<Sfx>,
}

impl Game {
    fn new(level: usize) -> Self {
        let level = level.clamp(1, 3);
        Self {
            level,
            speed: LEVEL_SPEED[level - 1],
            player_x: 0.0,
            player_y: 3.6,
            distance: 0.0,
            score: 0,
            hull: 100,
            actors: Vec::new(),
            bolts: Vec::new(),
            bursts: Vec::new(),
            spawn_timer: 0.5,
            obstacle_timer: 2.0,
            fire_timer: 0.0,
            flash_timer: 0.0,
            hit_flash: 0.0,
            feedback_timer: 0.0,
            feedback: String::new(),
            feedback_color: WHITE,
            threat_timer: 0.0,
            wall_timer: 0.0,
            reactor_spawned: false,
            end_state: None,
            rng: 0x6a09_e667_f3bc_c909,
            muted: false,
            audio_events: Vec::new(),
        }
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng as u32) as f32 / u32::MAX as f32
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.random()
    }

    fn course_offset(&self, z: f32) -> f32 {
        course_center(self.distance + z) - course_center(self.distance)
    }

    fn height_offset(&self, z: f32) -> f32 {
        course_height(self.distance + z) - course_height(self.distance)
    }

    fn player_world_x(&self) -> f32 {
        course_center(self.distance) + self.player_x
    }

    fn player_world_y(&self) -> f32 {
        course_height(self.distance) + self.player_y
    }

    fn sound(&mut self, cue: Sfx) {
        self.audio_events.push(cue);
    }

    fn project(&self, x: f32, y: f32, z: f32) -> Option<Vec2> {
        if z <= NEAR {
            return None;
        }
        let scale = 620.0 / (z + 7.0);
        Some(vec2(
            W / 2.0 + (x + self.course_offset(z) - self.player_x) * scale,
            H / 2.0 - (y + self.height_offset(z) - self.player_y) * scale,
        ))
    }

    fn project_world(&self, x: f32, y: f32, z: f32) -> Option<Vec2> {
        if z <= NEAR {
            return None;
        }
        let scale = 620.0 / (z + 7.0);
        Some(vec2(
            W / 2.0 + (x - self.player_world_x()) * scale,
            H / 2.0 - (y - self.player_world_y()) * scale,
        ))
    }

    fn feedback(&mut self, text: impl Into<String>, color: Color, duration: f32) {
        self.feedback = text.into();
        self.feedback_color = color;
        self.feedback_timer = duration;
    }

    fn damage(&mut self, amount: i32, reason: &str) {
        if self.end_state.is_some() {
            return;
        }
        self.hull = (self.hull - amount).max(0);
        self.hit_flash = 0.22;
        self.sound(Sfx::Damage);
        self.feedback(format!("{reason} -{amount}"), RED, 0.65);
        if self.hull == 0 {
            self.end_state = Some(EndState::Lost);
            self.sound(Sfx::Defeat);
        }
    }

    fn spawn(&mut self) {
        let kind = if self.random() < 0.36 {
            Kind::Fighter
        } else {
            Kind::Target
        };
        let x = self.range(-4.8, 4.8);
        let y = self.range(1.5, 6.7);
        let shot_timer = self.range(1.3, 2.5);
        self.actors.push(Actor {
            kind,
            x,
            y,
            z: 145.0,
            shot_timer,
        });
    }

    fn spawn_obstacle(&mut self) {
        let x = [-3.8, 0.0, 3.8][(self.random() * 3.0) as usize % 3];
        let y = if self.random() < 0.5 { 2.1 } else { 5.9 };
        self.actors.push(Actor {
            kind: Kind::Obstacle,
            x,
            y,
            z: 145.0,
            shot_timer: 0.0,
        });
    }

    fn fire(&mut self) {
        self.fire_timer = FIRE_INTERVAL;
        self.flash_timer = 0.09;
        self.sound(Sfx::Laser);
        let victim = self
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                let (rx, ry) = match a.kind {
                    Kind::Reactor => (0.48, 0.48),
                    Kind::Fighter => (1.0, 0.8),
                    Kind::Target => (0.72, 0.72),
                    Kind::Obstacle => (0.0, 0.0),
                };
                a.kind != Kind::Obstacle
                    && a.z > 2.0
                    && a.z < 125.0
                    && (a.x + self.course_offset(a.z) - self.player_x).abs() < rx
                    && (a.y + self.height_offset(a.z) - self.player_y).abs() < ry
            })
            .min_by(|(_, a), (_, b)| a.z.total_cmp(&b.z))
            .map(|(i, _)| i);
        if let Some(index) = victim {
            let actor = self.actors.remove(index);
            let points = match actor.kind {
                Kind::Target => 100,
                Kind::Fighter => 150,
                Kind::Reactor => 1000,
                Kind::Obstacle => 0,
            };
            self.score += points;
            self.sound(Sfx::Hit);
            if let Some(screen) = self.project(actor.x, actor.y, actor.z) {
                self.bursts.push(Burst {
                    screen,
                    age: 0.0,
                    color: if actor.kind == Kind::Reactor {
                        YELLOW
                    } else {
                        GREEN
                    },
                });
            }
            self.feedback(format!("HIT +{points}"), GREEN, 0.6);
            if actor.kind == Kind::Reactor {
                self.end_state = Some(EndState::Won);
                self.bolts.clear();
                self.sound(Sfx::Victory);
            }
        } else {
            self.feedback("MISS", DIM, 0.18);
        }
    }

    fn tick(&mut self, dt: f32, controls: Controls) {
        if self.end_state.is_some() {
            return;
        }
        let length = (controls.x * controls.x + controls.y * controls.y)
            .sqrt()
            .max(1.0);
        let old_center = course_center(self.distance);
        let old_height = course_height(self.distance);
        self.player_x += controls.x / length * PLAYER_SPEED * dt;
        self.player_y += controls.y / length * PLAYER_SPEED * dt;
        self.distance += self.speed * dt;
        self.player_x -= course_center(self.distance) - old_center;
        self.player_y -= course_height(self.distance) - old_height;
        self.wall_timer = (self.wall_timer - dt).max(0.0);
        let wall_reason = if self.player_x.abs() > 5.8 {
            Some("WALL SCRAPE")
        } else if self.player_y < 1.0 || self.player_y > 7.0 {
            Some("FLOOR / CEILING")
        } else {
            None
        };
        if let Some(reason) = wall_reason {
            self.player_x = self.player_x.clamp(-5.8, 5.8);
            self.player_y = self.player_y.clamp(1.0, 7.0);
            if self.wall_timer == 0.0 {
                self.damage(6, reason);
                self.wall_timer = 0.8;
            }
        }
        if self.end_state.is_some() {
            return;
        }
        self.fire_timer = (self.fire_timer - dt).max(0.0);
        self.flash_timer = (self.flash_timer - dt).max(0.0);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.feedback_timer = (self.feedback_timer - dt).max(0.0);
        self.threat_timer = (self.threat_timer - dt).max(0.0);
        for burst in &mut self.bursts {
            burst.age += dt;
        }
        self.bursts.retain(|b| b.age <= 0.42);

        if controls.fire && self.fire_timer == 0.0 {
            self.fire();
            if self.end_state.is_some() {
                return;
            }
        }
        if !self.reactor_spawned && self.distance >= REACTOR_DISTANCE {
            self.reactor_spawned = true;
            self.actors.clear();
            self.bolts.clear();
            self.actors.push(Actor {
                kind: Kind::Reactor,
                x: 0.0,
                y: 1.2,
                z: 145.0,
                shot_timer: 0.0,
            });
            self.feedback("FINAL TARGET AHEAD", YELLOW, 2.0);
        }
        if !self.reactor_spawned {
            self.spawn_timer -= dt;
            if self.spawn_timer <= 0.0 {
                self.spawn();
                self.spawn_timer += self.range(1.5, 2.4);
            }
            self.obstacle_timer -= dt;
            if self.obstacle_timer <= 0.0 {
                self.spawn_obstacle();
                let (low, high) = OBSTACLE_DELAY[self.level - 1];
                self.obstacle_timer += self.range(low, high);
            }
        }

        for index in (0..self.bolts.len()).rev() {
            let bolt = &mut self.bolts[index];
            bolt.z -= BOLT_SPEED * dt;
            let travel = (1.0 - bolt.z / bolt.start_z).clamp(0.0, 1.0);
            bolt.x += (bolt.aim_x - bolt.x) * travel;
            bolt.y += (bolt.aim_y - bolt.y) * travel;
            if bolt.z <= NEAR {
                let bolt = self.bolts.remove(index);
                if (self.player_world_x() - bolt.aim_x).abs() < 0.85
                    && (self.player_world_y() - bolt.aim_y).abs() < 0.85
                {
                    self.damage(8, "LASER HIT");
                }
            }
        }
        if self.end_state.is_some() {
            return;
        }

        for index in (0..self.actors.len()).rev() {
            let next_shot_delay = 1.3 + 0.9 * self.random();
            let actor_z = self.actors[index].z - self.speed * dt;
            let course_offset = self.course_offset(actor_z);
            let height_offset = self.height_offset(actor_z);
            let actor_world_center = course_center(self.distance + actor_z);
            let actor_world_height = course_height(self.distance + actor_z);
            let player_world_x = self.player_world_x();
            let player_world_y = self.player_world_y();
            let actor = &mut self.actors[index];
            actor.z -= self.speed * dt;
            if actor.kind == Kind::Fighter {
                let turn = 2.0 * dt;
                actor.x += (self.player_x - course_offset - actor.x).clamp(-turn, turn);
                actor.y += (self.player_y - height_offset - actor.y).clamp(-turn, turn);
                actor.shot_timer -= dt;
                if actor.shot_timer <= 0.0 && actor.z > 12.0 && actor.z < 100.0 {
                    if (actor.x + course_offset - self.player_x).abs() < 2.3
                        && (actor.y + height_offset - self.player_y).abs() < 2.0
                    {
                        self.bolts.push(Bolt {
                            x: actor_world_center + actor.x,
                            y: actor_world_height + actor.y,
                            z: actor.z,
                            aim_x: player_world_x,
                            aim_y: player_world_y,
                            start_z: actor.z,
                        });
                        self.threat_timer = 0.5;
                        self.audio_events.push(Sfx::Warning);
                    }
                    actor.shot_timer = next_shot_delay;
                }
            }
            if self.actors[index].z <= NEAR {
                let actor = self.actors.remove(index);
                match actor.kind {
                    Kind::Reactor => {
                        self.end_state = Some(EndState::Lost);
                        self.sound(Sfx::Defeat);
                        self.feedback("FINAL TARGET MISSED", RED, 2.0);
                    }
                    Kind::Target => self.damage(12, "TARGET MISSED"),
                    Kind::Obstacle
                        if (actor.x + course_offset - self.player_x).abs() < 1.25
                            && (actor.y + height_offset - self.player_y).abs() < 1.25 =>
                    {
                        self.damage(20, "OBSTACLE HIT")
                    }
                    Kind::Obstacle => {}
                    Kind::Fighter
                        if (actor.x + course_offset - self.player_x).abs() < 1.0
                            && (actor.y + height_offset - self.player_y).abs() < 1.0 =>
                    {
                        self.damage(18, "COLLISION")
                    }
                    Kind::Fighter => {}
                }
            }
            if self.end_state.is_some() {
                return;
            }
        }
    }

    fn line3d(&self, mut a: (f32, f32, f32), mut b: (f32, f32, f32), color: Color, width: f32) {
        if a.2 <= NEAR && b.2 <= NEAR {
            return;
        }
        if a.2 <= NEAR || b.2 <= NEAR {
            if a.2 <= NEAR {
                std::mem::swap(&mut a, &mut b);
            }
            let t = (a.2 - (NEAR + 0.01)) / (a.2 - b.2);
            b = (
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
                a.2 + (b.2 - a.2) * t,
            );
        }
        let steps = ((b.2 - a.2).abs() / 5.0).ceil().max(1.0) as usize;
        let mut previous = self.project(a.0, a.1, a.2);
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let point = (
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
                a.2 + (b.2 - a.2) * t,
            );
            let current = self.project(point.0, point.1, point.2);
            if let (Some(pa), Some(pb)) = (previous, current) {
                draw_line(pa.x, pa.y, pb.x, pb.y, width, color);
            }
            previous = current;
        }
    }

    fn trench(&self) {
        for x in [-7.0, 7.0] {
            for y in [0.0, 8.0] {
                self.line3d((x, y, 1.02), (x, y, 160.0), GREEN, 2.0);
            }
            for y in [1.5, 4.1, 6.7] {
                self.line3d((x, y, 1.02), (x, y, 160.0), DIM, 1.0);
            }
        }
        for x in [-4.5, 0.0, 4.5] {
            self.line3d((x, 0.0, 1.02), (x, 0.0, 160.0), DIM, 1.0);
        }
        let mut z = 9.0 - self.distance.rem_euclid(9.0);
        while z < 160.0 {
            let fade = (1.0 - z / 220.0).max(0.3);
            let color = Color::new(GREEN.r * fade, GREEN.g * fade, GREEN.b * fade, 1.0);
            self.line3d((-7.0, 0.0, z), (7.0, 0.0, z), color, 1.0);
            for x in [-7.0, 7.0] {
                self.line3d((x, 0.0, z), (x, 8.0, z), color, 1.0);
                let side = if x < 0.0 { 0.8 } else { -0.8 };
                for y in [1.5, 6.7] {
                    self.line3d((x, y, z), (x + side, y, z + 2.0), DIM, 1.0);
                }
            }
            z += 9.0;
        }
    }

    fn actor(&self, a: &Actor) {
        let Some(center) = self.project(a.x, a.y, a.z) else {
            return;
        };
        let scale = 620.0 / (a.z + 7.0);
        match a.kind {
            Kind::Obstacle => {
                for depth in [0.0, 2.0] {
                    let corners = [
                        (a.x - 1.0, a.y - 1.0, a.z + depth),
                        (a.x + 1.0, a.y - 1.0, a.z + depth),
                        (a.x + 1.0, a.y + 1.0, a.z + depth),
                        (a.x - 1.0, a.y + 1.0, a.z + depth),
                    ];
                    for i in 0..4 {
                        self.line3d(corners[i], corners[(i + 1) % 4], RED, 2.0);
                    }
                }
                for sx in [-1.0, 1.0] {
                    for sy in [-1.0, 1.0] {
                        self.line3d(
                            (a.x + sx, a.y + sy, a.z),
                            (a.x + sx, a.y + sy, a.z + 2.0),
                            YELLOW,
                            1.0,
                        );
                    }
                }
            }
            Kind::Target => {
                let radius = (0.9 * scale).max(3.0);
                if radius > 300.0 {
                    return;
                }
                draw_circle_lines(center.x, center.y, radius, 2.0, YELLOW);
                draw_line(
                    center.x - radius,
                    center.y,
                    center.x + radius,
                    center.y,
                    1.0,
                    YELLOW,
                );
                draw_line(
                    center.x,
                    center.y - radius,
                    center.x,
                    center.y + radius,
                    1.0,
                    YELLOW,
                );
            }
            Kind::Reactor => {
                let radius = (1.3 * scale).max(5.0);
                draw_circle_lines(center.x, center.y, radius + 8.0, 2.0, RED);
                draw_circle_lines(center.x, center.y, radius, 3.0, YELLOW);
                draw_circle_lines(center.x, center.y, (radius / 3.0).max(3.0), 2.0, RED);
                for direction in [
                    vec2(-1.0, 0.0),
                    vec2(1.0, 0.0),
                    vec2(0.0, -1.0),
                    vec2(0.0, 1.0),
                ] {
                    let p = center + direction * (radius + 12.0);
                    let q = center + direction * (radius + 23.0);
                    draw_line(p.x, p.y, q.x, q.y, 2.0, YELLOW);
                }
            }
            Kind::Fighter => {
                for side in [-1.0, 1.0] {
                    let sx = side * 1.5;
                    let shape = [
                        (sx - 0.55, -1.0),
                        (sx + 0.55, -1.0),
                        (sx + 0.85, 0.0),
                        (sx + 0.55, 1.0),
                        (sx - 0.55, 1.0),
                        (sx - 0.85, 0.0),
                    ];
                    for i in 0..shape.len() {
                        let p = shape[i];
                        let q = shape[(i + 1) % shape.len()];
                        self.line3d(
                            (a.x + p.0, a.y + p.1, a.z),
                            (a.x + q.0, a.y + q.1, a.z),
                            RED,
                            2.0,
                        );
                    }
                    self.line3d((a.x, a.y, a.z), (a.x + sx, a.y, a.z), RED, 2.0);
                }
                draw_circle_lines(center.x, center.y, (0.45 * scale).max(3.0), 2.0, RED);
            }
        }
    }

    fn draw(&self) {
        clear_background(BG);
        self.trench();
        let mut actors = self.actors.clone();
        actors.sort_by(|a, b| b.z.total_cmp(&a.z));
        for actor in &actors {
            self.actor(actor);
        }
        for bolt in &self.bolts {
            if let (Some(head), Some(tail)) = (
                self.project_world(bolt.x, bolt.y, bolt.z),
                self.project_world(bolt.x, bolt.y, bolt.z + 4.0),
            ) {
                draw_line(head.x, head.y, tail.x, tail.y, 4.0, RED);
                draw_circle(head.x, head.y, 3.0, YELLOW);
            }
        }
        for burst in &self.bursts {
            let radius = 5.0 + burst.age * 85.0;
            draw_circle_lines(burst.screen.x, burst.screen.y, radius, 2.0, burst.color);
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let direction = vec2(angle.cos(), angle.sin());
                let p = burst.screen + direction * radius * 0.55;
                let q = burst.screen + direction * radius;
                draw_line(p.x, p.y, q.x, q.y, 2.0, burst.color);
            }
        }
        self.hud();
        if self.hit_flash > 0.0 {
            draw_rectangle(
                0.0,
                0.0,
                W,
                H,
                Color::new(1.0, 0.1, 0.1, 0.29 * self.hit_flash / 0.22),
            );
        }
    }

    fn hud(&self) {
        draw_text(&format!("SCORE {:06}", self.score), 24.0, 39.0, 27.0, GREEN);
        draw_text(
            &format!("LEVEL {}  SPEED {}", self.level, self.speed as i32),
            24.0,
            86.0,
            19.0,
            DIM,
        );
        draw_text(
            &format!("HULL {:03}%", self.hull),
            W - 174.0,
            39.0,
            27.0,
            if self.hull <= 30 { RED } else { GREEN },
        );
        if self.reactor_spawned {
            draw_text("OBJECTIVE: HIT THE REACTOR CORE", 24.0, 62.0, 19.0, YELLOW);
        } else {
            draw_text(
                &format!(
                    "APPROACH {:03}%",
                    (self.distance / REACTOR_DISTANCE * 100.0).min(100.0) as i32
                ),
                24.0,
                62.0,
                19.0,
                DIM,
            );
            draw_text(
                "OBJECTIVE: SURVIVE TO THE FINAL TARGET",
                310.0,
                38.0,
                18.0,
                YELLOW,
            );
        }
        if self.threat_timer > 0.0 && self.end_state.is_none() {
            centered_text("INCOMING LASER - DODGE", H / 2.0 - 218.0, 27.0, RED);
        }
        let bend = course_center(self.distance + 45.0) - course_center(self.distance + 15.0);
        if bend.abs() > 0.8 && self.end_state.is_none() {
            draw_text(
                if bend > 0.0 {
                    "COURSE RIGHT"
                } else {
                    "COURSE LEFT"
                },
                W - 180.0,
                62.0,
                19.0,
                YELLOW,
            );
        }
        let climb = course_height(self.distance + 45.0) - course_height(self.distance + 15.0);
        if climb.abs() > 0.55 && self.end_state.is_none() {
            draw_text(
                if climb > 0.0 { "CLIMB" } else { "DESCEND" },
                W - 180.0,
                84.0,
                19.0,
                YELLOW,
            );
        }
        let center = vec2(W / 2.0, H / 2.0);
        let color = if self.feedback_timer > 0.0 && self.feedback.starts_with("HIT +") {
            GREEN
        } else {
            CYAN
        };
        draw_circle_lines(center.x, center.y, 15.0, 1.0, color);
        for (dx, dy) in [(-29.0, 0.0), (19.0, 0.0), (0.0, -29.0), (0.0, 19.0)] {
            let start = center + vec2(dx, dy);
            let end = start
                + if dx == 0.0 {
                    vec2(0.0, 10.0)
                } else {
                    vec2(10.0, 0.0)
                };
            draw_line(start.x, start.y, end.x, end.y, 2.0, color);
        }
        if self.feedback_timer > 0.0 && self.end_state.is_none() {
            centered_text(&self.feedback, H / 2.0 + 53.0, 19.0, self.feedback_color);
        }
        if self.flash_timer > 0.0 {
            for x in [W / 2.0 - 90.0, W / 2.0 + 90.0] {
                draw_line(x, H, center.x, center.y, 3.0, CYAN);
            }
        }
        centered_text(
            &format!(
                "WASD / ARROWS  MOVE   SPACE  FIRE   M  {}   R  RESTART   TAB  LEVELS   ESC  QUIT",
                if self.muted { "UNMUTE" } else { "MUTE" }
            ),
            H - 18.0,
            18.0,
            DIM,
        );
        if let Some(end) = self.end_state {
            draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, 0.75));
            let (title, color) = match end {
                EndState::Won => ("MISSION COMPLETE", GREEN),
                EndState::Lost => ("MISSION FAILED", RED),
            };
            centered_text(title, H / 2.0 - 27.0, 53.0, color);
            centered_text(
                &format!("FINAL SCORE {:06}", self.score),
                H / 2.0 + 22.0,
                27.0,
                YELLOW,
            );
            centered_text("R RESTART  |  TAB LEVELS", H / 2.0 + 63.0, 27.0, WHITE);
        }
    }
}

fn draw_menu(selected: usize) {
    let preview = Game::new(selected);
    clear_background(BG);
    preview.trench();
    draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, 0.80));
    centered_text("VECTOR TRENCH", 113.0, 53.0, GREEN);
    centered_text("SELECT A LEVEL", 164.0, 27.0, WHITE);
    let descriptions = [
        "TRAINING  |  SPEED 24  |  FEWER OBSTACLES",
        "ASSAULT   |  SPEED 31  |  MORE OBSTACLES",
        "ACE RUN   |  SPEED 39  |  DENSE OBSTACLES",
    ];
    for (i, description) in descriptions.iter().enumerate() {
        let level = i + 1;
        let y = 184.0 + i as f32 * 78.0;
        let color = if level == selected { GREEN } else { DIM };
        draw_rectangle_lines(174.0, y, 612.0, 58.0, 2.0, color);
        centered_text(
            &format!("{}   {}", level, description),
            y + 38.0,
            25.0,
            color,
        );
    }
    centered_text(
        "UP / DOWN  SELECT   ENTER  START   OR PRESS 1 / 2 / 3",
        462.0,
        19.0,
        CYAN,
    );
    centered_text(
        "STEER THROUGH LEFT / RIGHT TURNS AND CLIMBS / DESCENTS. DODGE THE RED BLOCKS.",
        510.0,
        18.0,
        WHITE,
    );
}

fn centered_text(text: &str, y: f32, size: f32, color: Color) {
    let width = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, (W - width) / 2.0, y, size, color);
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Vector Trench".to_owned(),
        window_width: W as i32,
        window_height: H as i32,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut selected_level = 1usize;
    let mut menu = true;
    let mut game = Game::new(selected_level);
    let sounds = Sounds::load().await;
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::M) {
            game.muted = !game.muted;
        }
        if is_key_pressed(KeyCode::Tab) {
            menu = true;
            let muted = game.muted;
            game = Game::new(selected_level);
            game.muted = muted;
        }
        if menu {
            if is_key_pressed(KeyCode::Up) {
                selected_level = if selected_level == 1 {
                    3
                } else {
                    selected_level - 1
                };
            }
            if is_key_pressed(KeyCode::Down) {
                selected_level = if selected_level == 3 {
                    1
                } else {
                    selected_level + 1
                };
            }
            let chosen = if is_key_pressed(KeyCode::Key1) {
                Some(1)
            } else if is_key_pressed(KeyCode::Key2) {
                Some(2)
            } else if is_key_pressed(KeyCode::Key3) {
                Some(3)
            } else if is_key_pressed(KeyCode::Enter) {
                Some(selected_level)
            } else {
                None
            };
            if let Some(level) = chosen {
                selected_level = level;
                let muted = game.muted;
                game = Game::new(level);
                game.muted = muted;
                menu = false;
            } else {
                draw_menu(selected_level);
                next_frame().await;
                continue;
            }
        }
        if game.end_state.is_some() {
            let chosen = if is_key_pressed(KeyCode::Key1) {
                Some(1)
            } else if is_key_pressed(KeyCode::Key2) {
                Some(2)
            } else if is_key_pressed(KeyCode::Key3) {
                Some(3)
            } else {
                None
            };
            if let Some(level) = chosen {
                selected_level = level;
                let muted = game.muted;
                game = Game::new(level);
                game.muted = muted;
            }
        }
        if is_key_pressed(KeyCode::R) {
            let muted = game.muted;
            game = Game::new(selected_level);
            game.muted = muted;
        }
        let x = (is_key_down(KeyCode::D) || is_key_down(KeyCode::Right)) as i32
            - (is_key_down(KeyCode::A) || is_key_down(KeyCode::Left)) as i32;
        let y = (is_key_down(KeyCode::W) || is_key_down(KeyCode::Up)) as i32
            - (is_key_down(KeyCode::S) || is_key_down(KeyCode::Down)) as i32;
        let controls = Controls {
            x: x as f32,
            y: y as f32,
            fire: is_key_down(KeyCode::Space),
        };
        game.tick(get_frame_time().min(0.05), controls);
        for cue in game.audio_events.drain(..) {
            if !game.muted {
                sounds.play(cue);
            }
        }
        game.draw();
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_hit_scores_and_miss_does_not() {
        let mut game = Game::new(1);
        let target_x = -game.course_offset(30.0);
        let target_y = 3.6 - game.height_offset(30.0);
        game.actors.push(Actor {
            kind: Kind::Target,
            x: target_x,
            y: target_y,
            z: 30.0,
            shot_timer: 0.0,
        });
        game.fire();
        assert_eq!(game.score, 100);
        assert!(game.actors.is_empty());
        assert!(game.audio_events.contains(&Sfx::Hit));
        game.fire();
        assert_eq!(game.score, 100);
        assert_eq!(game.feedback, "MISS");
    }

    #[test]
    fn reactor_requires_low_aim_and_ends_mission() {
        let mut game = Game::new(1);
        game.distance = REACTOR_DISTANCE;
        game.spawn_timer = 999.0;
        game.tick(0.01, Controls::default());
        assert!(game.reactor_spawned);
        assert_eq!(game.actors[0].kind, Kind::Reactor);
        game.actors[0].z = 30.0;
        game.fire();
        assert_eq!(game.end_state, None);
        game.player_y = 1.2 + game.height_offset(30.0);
        game.player_x = game.course_offset(30.0);
        game.fire();
        assert_eq!(game.end_state, Some(EndState::Won));
        assert_eq!(game.score, 1000);
    }

    #[test]
    fn reactor_miss_and_dodgeable_bolt() {
        let mut game = Game::new(1);
        game.bolts.push(Bolt {
            x: 0.0,
            y: 3.6,
            z: 20.0,
            aim_x: 0.0,
            aim_y: 3.6,
            start_z: 20.0,
        });
        game.player_x = 3.0;
        game.spawn_timer = 999.0;
        for _ in 0..20 {
            game.tick(0.02, Controls::default());
        }
        assert_eq!(game.hull, 100);
        game.distance = REACTOR_DISTANCE;
        game.tick(0.01, Controls::default());
        game.actors[0].z = 1.1;
        game.tick(0.01, Controls::default());
        assert_eq!(game.end_state, Some(EndState::Lost));
    }

    #[test]
    fn bends_and_climbs_shift_the_trench_around_a_stationary_ship() {
        let mut game = Game::new(1);
        let world_x = game.player_world_x();
        let world_y = game.player_world_y();
        game.spawn_timer = 999.0;
        game.obstacle_timer = 999.0;
        game.tick(0.5, Controls::default());
        assert!((game.player_world_x() - world_x).abs() < 0.01);
        assert!((game.player_world_y() - world_y).abs() < 0.01);
        assert!(game.player_x.abs() > 0.1);
        assert!(game.player_y < 3.6);
        assert!(course_center(100.0) > course_center(0.0));
        assert!(course_center(300.0) < course_center(100.0));
        assert!(course_height(100.0) > course_height(0.0));
        assert!(course_height(300.0) < course_height(100.0));
    }

    #[test]
    fn levels_change_speed_and_obstacle_collision_damages_hull() {
        assert_eq!(Game::new(1).speed, 24.0);
        assert_eq!(Game::new(2).speed, 31.0);
        let mut game = Game::new(3);
        assert_eq!(game.speed, 39.0);
        assert!(OBSTACLE_DELAY[2].1 < OBSTACLE_DELAY[0].0);
        game.spawn_timer = 999.0;
        game.obstacle_timer = 999.0;
        game.actors.push(Actor {
            kind: Kind::Obstacle,
            x: game.player_x - game.course_offset(1.1),
            y: game.player_y - game.height_offset(1.1),
            z: 1.1,
            shot_timer: 0.0,
        });
        game.tick(0.01, Controls::default());
        assert_eq!(game.hull, 80);
    }
}
