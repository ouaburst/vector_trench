"""Vector Trench: a small, self-contained Pygame arcade flight game."""

from __future__ import annotations

import math
import random
from dataclasses import dataclass
from pathlib import Path

import pygame


WIDTH, HEIGHT = 960, 600
FPS = 60
FOCAL = 620
NEAR = 1.0
TRENCH_HALF_WIDTH = 7.0
TRENCH_HEIGHT = 8.0
PLAYER_SPEED = 7.0
FORWARD_SPEED = 27.0
FIRE_INTERVAL = 0.18
REACTOR_DISTANCE = 560.0
REACTOR_BONUS = 1000
BOLT_SPEED = 105.0

BLACK = (2, 8, 12)
GREEN = (78, 236, 168)
DIM_GREEN = (31, 109, 93)
CYAN = (73, 217, 243)
RED = (255, 88, 92)
YELLOW = (255, 210, 99)
WHITE = (207, 244, 230)


def course_center(distance: float) -> float:
    """The trench center drifts through broad and short alternating bends."""
    return 4.0 * math.sin(distance / 85.0) + 1.7 * math.sin(distance / 33.0)


@dataclass
class Actor:
    kind: str  # "target", "fighter", or "reactor"
    x: float
    y: float
    z: float
    phase: float = 0.0
    shot_timer: float = 0.0


@dataclass
class EnemyBolt:
    x: float
    y: float
    z: float
    aim_x: float
    aim_y: float
    start_z: float


@dataclass
class Burst:
    x: int
    y: int
    age: float
    color: tuple[int, int, int]


class Game:
    def __init__(self) -> None:
        pygame.init()
        self.screen = pygame.display.set_mode((WIDTH, HEIGHT))
        pygame.display.set_caption("Vector Trench")
        self.clock = pygame.time.Clock()
        self.font = pygame.font.SysFont("monospace", 21, bold=True)
        self.small_font = pygame.font.SysFont("monospace", 15)
        self.large_font = pygame.font.SysFont("monospace", 48, bold=True)
        self.rng = random.Random()
        self.sounds: dict[str, pygame.mixer.Sound] = {}
        self.muted = False
        try:
            if not pygame.mixer.get_init():
                pygame.mixer.init()
            for name in ("laser", "hit", "damage", "warning", "victory", "defeat"):
                self.sounds[name] = pygame.mixer.Sound(str(Path(__file__).parent / "assets" / f"{name}.wav"))
        except (pygame.error, FileNotFoundError):
            # Flight remains playable on machines without an audio device.
            self.sounds.clear()
        self.running = True
        self.reset()

    def play_sound(self, name: str) -> None:
        if not self.muted and name in self.sounds:
            self.sounds[name].play()

    def reset(self) -> None:
        self.player_x = 0.0
        self.player_y = 3.6
        self.distance = 0.0
        self.score = 0
        self.hull = 100
        self.actors: list[Actor] = []
        self.bolts: list[EnemyBolt] = []
        self.bursts: list[Burst] = []
        self.spawn_timer = 0.5
        self.fire_timer = 0.0
        self.flash_timer = 0.0
        self.hit_flash = 0.0
        self.feedback_timer = 0.0
        self.feedback_text = ""
        self.feedback_color = WHITE
        self.threat_timer = 0.0
        self.wall_timer = 0.0
        self.reactor_spawned = False
        self.end_state: str | None = None

    def course_offset(self, z: float) -> float:
        return course_center(self.distance + z) - course_center(self.distance)

    def player_world_x(self) -> float:
        return course_center(self.distance) + self.player_x

    def project(self, x: float, y: float, z: float) -> tuple[int, int] | None:
        if z <= NEAR:
            return None
        scale = FOCAL / (z + 7.0)
        return (int(WIDTH / 2 + (x + self.course_offset(z) - self.player_x) * scale),
                int(HEIGHT / 2 - (y - self.player_y) * scale))

    def project_world(self, x: float, y: float, z: float) -> tuple[int, int] | None:
        if z <= NEAR:
            return None
        scale = FOCAL / (z + 7.0)
        return (int(WIDTH / 2 + (x - self.player_world_x()) * scale),
                int(HEIGHT / 2 - (y - self.player_y) * scale))

    def line3d(self, a: tuple[float, float, float],
               b: tuple[float, float, float], color: tuple[int, int, int],
               width: int = 1) -> None:
        # Clip a segment at the near plane so no line jumps behind the camera.
        if a[2] <= NEAR and b[2] <= NEAR:
            return
        if a[2] <= NEAR or b[2] <= NEAR:
            if a[2] <= NEAR:
                a, b = b, a
            t = (a[2] - (NEAR + 0.01)) / (a[2] - b[2])
            b = tuple(a[i] + (b[i] - a[i]) * t for i in range(3))
        # Long edges are sampled so they bend with the course instead of
        # cutting straight across its turns.
        steps = max(1, math.ceil(abs(b[2] - a[2]) / 5.0))
        previous = self.project(*a)
        for step in range(1, steps + 1):
            t = step / steps
            point = tuple(a[i] + (b[i] - a[i]) * t for i in range(3))
            current = self.project(*point)
            if previous is not None and current is not None:
                pygame.draw.line(self.screen, color, previous, current, width)
            previous = current

    def draw_trench(self) -> None:
        half = TRENCH_HALF_WIDTH
        height = TRENCH_HEIGHT
        # Continuous edges give the player a strong sense of motion.
        for x in (-half, half):
            for y in (0.0, height):
                self.line3d((x, y, NEAR + 0.02), (x, y, 160), GREEN, 2)
            for y in (1.5, 4.1, 6.7):
                self.line3d((x, y, NEAR + 0.02), (x, y, 160), DIM_GREEN)
        for x in (-4.5, 0.0, 4.5):
            self.line3d((x, 0, NEAR + 0.02), (x, 0, 160), DIM_GREEN)

        spacing = 9.0
        first = spacing - (self.distance % spacing)
        z = first
        while z < 160:
            fade = max(0.3, 1.0 - z / 220)
            color = tuple(int(c * fade) for c in GREEN)
            self.line3d((-half, 0, z), (half, 0, z), color)
            for x in (-half, half):
                self.line3d((x, 0, z), (x, height, z), color)
                # Short wall panels add detail without external assets.
                side = 0.8 if x < 0 else -0.8
                self.line3d((x, 1.5, z), (x + side, 1.5, z + 2), DIM_GREEN)
                self.line3d((x, 6.7, z), (x + side, 6.7, z + 2), DIM_GREEN)
            z += spacing

    def draw_actor(self, actor: Actor) -> None:
        center = self.project(actor.x, actor.y, actor.z)
        if center is None:
            return
        scale = FOCAL / (actor.z + 7.0)
        if actor.kind in ("target", "reactor"):
            if actor.kind == "reactor":
                radius = max(5, int(1.3 * scale))
                pygame.draw.circle(self.screen, RED, center, radius + 8, 2)
                pygame.draw.circle(self.screen, YELLOW, center, radius, 3)
                pygame.draw.circle(self.screen, RED, center, max(3, radius // 3), 2)
                for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    pygame.draw.line(self.screen, YELLOW,
                                     (center[0] + dx * (radius + 12), center[1] + dy * (radius + 12)),
                                     (center[0] + dx * (radius + 23), center[1] + dy * (radius + 23)), 2)
                return
            radius = max(3, int(0.9 * scale))
            if radius > 300:
                return
            pygame.draw.circle(self.screen, YELLOW, center, radius, 2)
            pygame.draw.line(self.screen, YELLOW,
                             (center[0] - radius, center[1]),
                             (center[0] + radius, center[1]))
            pygame.draw.line(self.screen, YELLOW,
                             (center[0], center[1] - radius),
                             (center[0], center[1] + radius))
            return

        # Twin hexagonal wings and a cockpit, all drawn in world space.
        def shape(points: list[tuple[float, float]], color: tuple[int, int, int]) -> None:
            projected = [self.project(actor.x + x, actor.y + y, actor.z) for x, y in points]
            if all(p is not None for p in projected):
                pygame.draw.lines(self.screen, color, True, projected, 2)

        for side in (-1, 1):
            sx = side * 1.5
            shape([(sx - 0.55, -1.0), (sx + 0.55, -1.0),
                   (sx + 0.85, 0), (sx + 0.55, 1.0),
                   (sx - 0.55, 1.0), (sx - 0.85, 0)], RED)
            self.line3d((actor.x, actor.y, actor.z),
                        (actor.x + sx, actor.y, actor.z), RED, 2)
        pygame.draw.circle(self.screen, RED, center, max(3, int(0.45 * scale)), 2)

    def damage(self, amount: int, reason: str = "HULL HIT") -> None:
        if self.end_state is not None:
            return
        self.hull = max(0, self.hull - amount)
        self.hit_flash = 0.22
        self.play_sound("damage")
        self.feedback_text = f"{reason} -{amount}"
        self.feedback_color = RED
        self.feedback_timer = 0.65
        if self.hull == 0:
            self.end_state = "lost"
            self.play_sound("defeat")

    def burst(self, actor: Actor, color: tuple[int, int, int]) -> None:
        center = self.project(actor.x, actor.y, actor.z)
        if center is not None:
            self.bursts.append(Burst(*center, 0.0, color))

    def spawn(self) -> None:
        kind = "fighter" if self.rng.random() < 0.36 else "target"
        self.actors.append(Actor(
            kind=kind,
            x=self.rng.uniform(-4.8, 4.8),
            y=self.rng.uniform(1.5, 6.7),
            z=145.0,
            phase=self.rng.uniform(0, 2 * math.pi),
            shot_timer=self.rng.uniform(1.3, 2.5),
        ))

    def fire(self) -> None:
        self.fire_timer = FIRE_INTERVAL
        self.flash_timer = 0.09
        self.play_sound("laser")
        # The center reticle is a forward ray through the player's position.
        candidates = [a for a in self.actors if 2 < a.z < 125
                      and abs(a.x + self.course_offset(a.z) - self.player_x) < (0.48 if a.kind == "reactor" else 1.0 if a.kind == "fighter" else 0.72)
                      and abs(a.y - self.player_y) < (0.48 if a.kind == "reactor" else 0.8 if a.kind == "fighter" else 0.72)]
        if candidates:
            victim = min(candidates, key=lambda a: a.z)
            self.actors.remove(victim)
            value = REACTOR_BONUS if victim.kind == "reactor" else 150 if victim.kind == "fighter" else 100
            self.score += value
            self.play_sound("hit")
            self.burst(victim, YELLOW if victim.kind == "reactor" else GREEN)
            self.feedback_text = f"HIT +{value}"
            self.feedback_color = GREEN
            self.feedback_timer = 0.6
            if victim.kind == "reactor":
                self.end_state = "won"
                self.bolts.clear()
                self.play_sound("victory")
        else:
            self.feedback_text = "MISS"
            self.feedback_color = DIM_GREEN
            self.feedback_timer = 0.18

    def update(self, dt: float) -> None:
        keys = pygame.key.get_pressed()
        if self.end_state is not None:
            return
        dx = int(keys[pygame.K_d] or keys[pygame.K_RIGHT]) - int(keys[pygame.K_a] or keys[pygame.K_LEFT])
        dy = int(keys[pygame.K_w] or keys[pygame.K_UP]) - int(keys[pygame.K_s] or keys[pygame.K_DOWN])
        length = math.hypot(dx, dy) or 1.0
        old_center = course_center(self.distance)
        self.player_x += PLAYER_SPEED * dx / length * dt
        self.player_y = max(1.0, min(7.0, self.player_y + PLAYER_SPEED * dy / length * dt))
        self.distance += FORWARD_SPEED * dt
        self.player_x -= course_center(self.distance) - old_center
        self.wall_timer = max(0.0, self.wall_timer - dt)
        if abs(self.player_x) > 5.8:
            self.player_x = max(-5.8, min(5.8, self.player_x))
            if self.wall_timer == 0:
                self.damage(6, "WALL SCRAPE")
                self.wall_timer = 0.8
        self.fire_timer = max(0.0, self.fire_timer - dt)
        self.flash_timer = max(0.0, self.flash_timer - dt)
        self.hit_flash = max(0.0, self.hit_flash - dt)
        self.feedback_timer = max(0.0, self.feedback_timer - dt)
        self.threat_timer = max(0.0, self.threat_timer - dt)
        for burst in self.bursts[:]:
            burst.age += dt
            if burst.age > 0.42:
                self.bursts.remove(burst)
        if keys[pygame.K_SPACE] and self.fire_timer == 0:
            self.fire()
            if self.end_state is not None:
                return

        if not self.reactor_spawned and self.distance >= REACTOR_DISTANCE:
            self.reactor_spawned = True
            self.actors.clear()
            self.bolts.clear()
            # The core sits low in the trench, requiring a deliberate descent.
            self.actors.append(Actor("reactor", 0.0, 1.2, 145.0))
            self.feedback_text = "FINAL TARGET AHEAD"
            self.feedback_color = YELLOW
            self.feedback_timer = 2.0

        if not self.reactor_spawned:
            self.spawn_timer -= dt
            if self.spawn_timer <= 0:
                self.spawn()
                self.spawn_timer += self.rng.uniform(1.5, 2.4)

        for bolt in self.bolts[:]:
            bolt.z -= BOLT_SPEED * dt
            travel = min(1.0, 1.0 - bolt.z / bolt.start_z)
            bolt.x += (bolt.aim_x - bolt.x) * travel
            bolt.y += (bolt.aim_y - bolt.y) * travel
            if bolt.z <= NEAR:
                self.bolts.remove(bolt)
                if abs(self.player_world_x() - bolt.aim_x) < 0.85 and abs(self.player_y - bolt.aim_y) < 0.85:
                    self.damage(8, "LASER HIT")

        for actor in self.actors[:]:
            actor.z -= FORWARD_SPEED * dt
            if actor.kind == "fighter":
                # Steer toward the player with bounded turn speed; firing requires alignment.
                turn = 2.0 * dt
                target_x = self.player_x - self.course_offset(actor.z)
                actor.x += max(-turn, min(turn, target_x - actor.x))
                actor.y += max(-turn, min(turn, self.player_y - actor.y))
                actor.shot_timer -= dt
                if actor.shot_timer <= 0 and 12 < actor.z < 100:
                    if abs(actor.x + self.course_offset(actor.z) - self.player_x) < 2.3 and abs(actor.y - self.player_y) < 2.0:
                        self.bolts.append(EnemyBolt(course_center(self.distance + actor.z) + actor.x,
                                                    actor.y, actor.z,
                                                    self.player_world_x(), self.player_y, actor.z))
                        self.threat_timer = 0.5
                        self.play_sound("warning")
                    actor.shot_timer = self.rng.uniform(1.3, 2.2)
            if actor.z <= NEAR:
                self.actors.remove(actor)
                if actor.kind == "reactor":
                    self.end_state = "lost"
                    self.play_sound("defeat")
                    self.feedback_text = "FINAL TARGET MISSED"
                    self.feedback_color = RED
                    self.feedback_timer = 2.0
                elif actor.kind == "target":
                    self.damage(12, "TARGET MISSED")
                elif abs(actor.x + self.course_offset(actor.z) - self.player_x) < 1.0 and abs(actor.y - self.player_y) < 1.0:
                    self.damage(18, "COLLISION")

    def draw_bolt(self, bolt: EnemyBolt) -> None:
        head = self.project_world(bolt.x, bolt.y, bolt.z)
        tail = self.project_world(bolt.x, bolt.y, bolt.z + 4)
        if head is not None and tail is not None:
            pygame.draw.line(self.screen, RED, head, tail, 4)
            pygame.draw.circle(self.screen, YELLOW, head, 3)

    def draw_bursts(self) -> None:
        for burst in self.bursts:
            radius = int(5 + burst.age * 85)
            pygame.draw.circle(self.screen, burst.color, (burst.x, burst.y), radius, 2)
            for angle in range(0, 360, 45):
                direction = math.radians(angle)
                inner = radius * 0.55
                start = (int(burst.x + math.cos(direction) * inner),
                         int(burst.y + math.sin(direction) * inner))
                end = (int(burst.x + math.cos(direction) * radius),
                       int(burst.y + math.sin(direction) * radius))
                pygame.draw.line(self.screen, burst.color, start, end, 2)

    def draw_hud(self) -> None:
        def label(text: str, x: int, y: int, color=WHITE, font=None) -> None:
            self.screen.blit((font or self.font).render(text, True, color), (x, y))

        label(f"SCORE {self.score:06d}", 24, 18, GREEN)
        label(f"HULL {self.hull:03d}%", WIDTH - 166, 18,
              RED if self.hull <= 30 else GREEN)
        if not self.reactor_spawned:
            label(f"APPROACH {int(min(self.distance / REACTOR_DISTANCE, 1) * 100):03d}%", 24, 46,
                  DIM_GREEN, self.small_font)
            label("OBJECTIVE: SURVIVE TO THE FINAL TARGET", 315, 19, YELLOW, self.small_font)
        else:
            label("OBJECTIVE: HIT THE REACTOR CORE", 24, 46, YELLOW, self.small_font)
        if self.threat_timer > 0 and self.end_state is None:
            warning = self.font.render("INCOMING LASER - DODGE", True, RED)
            self.screen.blit(warning, warning.get_rect(center=(WIDTH // 2, 75)))
        bend = course_center(self.distance + 45) - course_center(self.distance + 15)
        if abs(bend) > 0.8 and self.end_state is None:
            label("COURSE RIGHT" if bend > 0 else "COURSE LEFT", WIDTH - 178, 48,
                  YELLOW, self.small_font)
        cx, cy = WIDTH // 2, HEIGHT // 2
        reticle_color = GREEN if self.feedback_timer > 0 and self.feedback_text.startswith("HIT +") else CYAN
        pygame.draw.circle(self.screen, reticle_color, (cx, cy), 15, 1)
        for sx, sy, ex, ey in ((-29, 0, -19, 0), (19, 0, 29, 0),
                              (0, -29, 0, -19), (0, 19, 0, 29)):
            pygame.draw.line(self.screen, reticle_color, (cx + sx, cy + sy), (cx + ex, cy + ey), 2)
        if self.feedback_timer > 0 and self.end_state is None:
            feedback = self.small_font.render(self.feedback_text, True, self.feedback_color)
            self.screen.blit(feedback, feedback.get_rect(midtop=(cx, cy + 34)))
        if self.flash_timer > 0:
            pygame.draw.line(self.screen, CYAN, (WIDTH // 2 - 90, HEIGHT), (cx, cy), 3)
            pygame.draw.line(self.screen, CYAN, (WIDTH // 2 + 90, HEIGHT), (cx, cy), 3)
        label(f"WASD / ARROWS  MOVE    SPACE  FIRE    M  {'UNMUTE' if self.muted else 'MUTE'}    R  RESTART    ESC  QUIT",
              105, HEIGHT - 29, DIM_GREEN, self.small_font)

        if self.end_state is not None:
            overlay = pygame.Surface((WIDTH, HEIGHT), pygame.SRCALPHA)
            overlay.fill((0, 0, 0, 190))
            self.screen.blit(overlay, (0, 0))
            won = self.end_state == "won"
            title = self.large_font.render("MISSION COMPLETE" if won else "MISSION FAILED",
                                           True, GREEN if won else RED)
            self.screen.blit(title, title.get_rect(center=(WIDTH // 2, HEIGHT // 2 - 28)))
            result = self.font.render(f"FINAL SCORE {self.score:06d}", True, YELLOW)
            self.screen.blit(result, result.get_rect(center=(WIDTH // 2, HEIGHT // 2 + 18)))
            prompt = self.font.render("PRESS R TO RESTART", True, WHITE)
            self.screen.blit(prompt, prompt.get_rect(center=(WIDTH // 2, HEIGHT // 2 + 60)))

    def draw(self) -> None:
        self.screen.fill(BLACK)
        self.draw_trench()
        for actor in sorted(self.actors, key=lambda a: a.z, reverse=True):
            self.draw_actor(actor)
        for bolt in self.bolts:
            self.draw_bolt(bolt)
        self.draw_bursts()
        self.draw_hud()
        if self.hit_flash > 0:
            overlay = pygame.Surface((WIDTH, HEIGHT), pygame.SRCALPHA)
            overlay.fill((255, 30, 30, int(75 * self.hit_flash / 0.22)))
            self.screen.blit(overlay, (0, 0))
        pygame.display.flip()

    def run(self) -> None:
        while self.running:
            dt = min(self.clock.tick(FPS) / 1000.0, 0.05)
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    self.running = False
                elif event.type == pygame.KEYDOWN:
                    if event.key == pygame.K_ESCAPE:
                        self.running = False
                    elif event.key == pygame.K_r:
                        self.reset()
                    elif event.key == pygame.K_m:
                        self.muted = not self.muted
            self.update(dt)
            self.draw()
        pygame.quit()


if __name__ == "__main__":
    Game().run()
