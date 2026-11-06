use crate::math::*;
use crate::render::Canvas;
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Title,
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Drifter,
    Seeker,
    Snipper,
    Spinner,
}

impl Kind {
    fn color(self) -> Col {
        match self {
            Kind::Drifter => rgb(0.15, 1.0, 0.55),
            Kind::Seeker => rgb(1.0, 0.2, 0.85),
            Kind::Snipper => rgb(1.0, 0.18, 0.12),
            Kind::Spinner => rgb(1.0, 0.6, 0.08),
        }
    }
    fn radius(self) -> f32 {
        match self {
            Kind::Drifter => 3.0,
            Kind::Seeker => 3.2,
            Kind::Snipper => 3.0,
            Kind::Spinner => 4.6,
        }
    }
    fn points(self) -> u64 {
        match self {
            Kind::Drifter => 50,
            Kind::Seeker => 100,
            Kind::Snipper => 150,
            Kind::Spinner => 250,
        }
    }
}

struct Enemy {
    kind: Kind,
    pos: V2,
    vel: V2,
    ang: f32,
    spin: f32,
    spawn: f32,
    fire_t: f32,
    cut_cd: f32,
    age: f32,
}

const SPAWN_TIME: f32 = 0.75;

struct Bullet {
    pos: V2,
    vel: V2,
    life: f32,
}

#[derive(Clone, Copy)]
enum PKind {
    Spark,
    Dot,
    Glow(f32),
}

struct Particle {
    pos: V2,
    vel: V2,
    life: f32,
    max: f32,
    col: Col,
    drag: f32,
    kind: PKind,
}

struct Gem {
    pos: V2,
    vel: V2,
    age: f32,
}

struct Ring {
    pos: V2,
    r: f32,
    speed: f32,
    life: f32,
    max: f32,
    col: Col,
}

struct Popup {
    pos: V2,
    text: String,
    col: Col,
    life: f32,
    max: f32,
    scale: i32,
    rainbow: bool,
}

struct LoopFx {
    pts: Vec<V2>,
    center: V2,
    life: f32,
    max: f32,
    col: Col,
}

struct Bolt {
    a: V2,
    b: V2,
    life: f32,
    col: Col,
}

struct Star {
    pos: V2,
    depth: f32,
    phase: f32,
    col: Col,
}

struct Player {
    pos: V2,
    ang: f32,
    target: f32,
    dash: f32,
    energy: f32,
    invuln: f32,
    alive: bool,
    trail: VecDeque<V2>,
    trail_len: f32,
}

struct Grid {
    cols: usize,
    rows: usize,
    rest: Vec<V2>,
    pos: Vec<V2>,
    vel: Vec<V2>,
    disp: Vec<V2>,
}

impl Grid {
    fn new(w: f32, h: f32) -> Grid {
        let spacing = 7.0;
        let cols = ((w / spacing).round() as usize).max(2) + 1;
        let rows = ((h / spacing).round() as usize).max(2) + 1;
        let sx = (w - 1.0) / (cols - 1) as f32;
        let sy = (h - 1.0) / (rows - 1) as f32;
        let mut rest = Vec::with_capacity(cols * rows);
        for y in 0..rows {
            for x in 0..cols {
                rest.push(v2(0.5 + x as f32 * sx, 0.5 + y as f32 * sy));
            }
        }
        Grid { cols, rows, pos: rest.clone(), vel: vec![V2::default(); rest.len()], disp: vec![V2::default(); rest.len()], rest }
    }

    fn update(&mut self, dt: f32) {
        let steps = 2;
        let h = dt / steps as f32;
        let damp = (-2.6 * h).exp();
        for _ in 0..steps {
            for i in 0..self.pos.len() {
                self.disp[i] = self.pos[i] - self.rest[i];
            }
            for y in 0..self.rows {
                for x in 0..self.cols {
                    let i = y * self.cols + x;
                    if x == 0 || y == 0 || x == self.cols - 1 || y == self.rows - 1 {
                        self.pos[i] = self.rest[i];
                        self.vel[i] = V2::default();
                        continue;
                    }
                    let d = self.disp[i];
                    let nb = (self.disp[i - 1] + self.disp[i + 1] + self.disp[i - self.cols] + self.disp[i + self.cols]) * 0.25;
                    let acc = -d * 10.0 + (nb - d) * 70.0;
                    self.vel[i] = (self.vel[i] + acc * h) * damp;
                    self.pos[i] += self.vel[i] * h;
                }
            }
        }
    }

    fn impulse(&mut self, p: V2, radius: f32, strength: f32) {
        for i in 0..self.pos.len() {
            let v = self.pos[i] - p;
            let d = v.len();
            if d < radius && d > 0.01 {
                let f = 1.0 - d / radius;
                self.vel[i] += v * (1.0 / d) * (strength * f * f);
            }
        }
    }

    fn render(&self, cv: &mut Canvas, light: V2, light_col: Col) {
        let base = rgb(0.022, 0.035, 0.095);
        let hot = rgb(0.32, 0.14, 0.62);
        let col_at = |i: usize| -> Col {
            let d = (self.pos[i] - self.rest[i]).len();
            let e = (d / 4.0).min(1.0);
            let l2 = (self.pos[i] - light).len2();
            let lf = (-l2 / 500.0).exp();
            base.lerp(hot, e) + light_col * (lf * 0.22)
        };
        let cols: Vec<Col> = (0..self.pos.len()).map(col_at).collect();
        for y in 0..self.rows {
            for x in 0..self.cols {
                let i = y * self.cols + x;
                if x + 1 < self.cols {
                    cv.line(self.pos[i], self.pos[i + 1], cols[i], cols[i + 1]);
                }
                if y + 1 < self.rows {
                    cv.line(self.pos[i], self.pos[i + self.cols], cols[i], cols[i + self.cols]);
                }
            }
        }
    }
}

pub struct Game {
    pub w: f32,
    pub h: f32,
    pub mode: Mode,
    pub quit: bool,
    rng: Rng,
    t: f32,
    gt: f32,
    player: Player,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    particles: Vec<Particle>,
    gems: Vec<Gem>,
    rings: Vec<Ring>,
    popups: Vec<Popup>,
    loops: Vec<LoopFx>,
    bolts: Vec<Bolt>,
    grid: Grid,
    stars: Vec<Star>,
    nebula: Vec<Col>,
    score: u64,
    shown_score: f64,
    best: u64,
    new_best: bool,
    mult: u32,
    mult_timer: f32,
    mult_decay: f32,
    lives: u32,
    spawn_t: f32,
    swarm_t: f32,
    gameover_t: f32,
    respawn_t: f32,
    shake: f32,
    hitstop: f32,
    slowmo: f32,
    flash: f32,
    flash_col: Col,
    aberr: f32,
    held: [bool; 4],
    held_mode: bool,
    last_press: Option<(usize, f32)>,
    auto_dir: f32,
        kills: u32,
    best_loop: u32,
    loops_closed: u32,
    hearts_flash: f32,
    bot: bool,
    bot_center: Option<V2>,
    bot_r: f32,
    bot_t: f32,
    bot_loops: u32,
    god: bool,
}

const DIRS: [V2; 4] = [v2(0.0, -1.0), v2(0.0, 1.0), v2(-1.0, 0.0), v2(1.0, 0.0)];

fn best_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".neon_lasso_best"))
}

fn fit_scale(s: &str, max_w: f32, max_scale: i32) -> i32 {
    let tw = crate::font::text_width(s).max(1) as f32;
    ((max_w / tw).floor() as i32).clamp(1, max_scale)
}

fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl Game {
    pub fn new(w: f32, h: f32, held_mode: bool) -> Game {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        let best = best_path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let mut g = Game {
            w,
            h,
            mode: Mode::Title,
            quit: false,
            rng: Rng::new(seed),
            t: 0.0,
            gt: 0.0,
            player: Player {
                pos: v2(w / 2.0, h / 2.0),
                ang: -PI / 2.0,
                target: -PI / 2.0,
                dash: 0.0,
                energy: 1.0,
                invuln: 0.0,
                alive: true,
                trail: VecDeque::new(),
                trail_len: 0.0,
            },
            enemies: vec![],
            bullets: vec![],
            particles: vec![],
            gems: vec![],
            rings: vec![],
            popups: vec![],
            loops: vec![],
            bolts: vec![],
            grid: Grid::new(w, h),
            stars: vec![],
            nebula: vec![],
            score: 0,
            shown_score: 0.0,
            best,
            new_best: false,
            mult: 1,
            mult_timer: 0.0,
            mult_decay: 0.0,
            lives: 3,
            spawn_t: 0.0,
            swarm_t: 0.0,
            gameover_t: 0.0,
            respawn_t: 0.0,
            shake: 0.0,
            hitstop: 0.0,
            slowmo: 0.0,
            flash: 0.0,
            flash_col: WHITE,
            aberr: 0.0,
            held: [false; 4],
            held_mode,
            last_press: None,
            auto_dir: 1.0,
                        kills: 0,
            best_loop: 0,
            loops_closed: 0,
            hearts_flash: 0.0,
            bot: std::env::var_os("NEON_LASSO_BOT").is_some(),
            bot_center: None,
            bot_r: 12.0,
            bot_t: 0.0,
            bot_loops: 0,
            god: std::env::var("NEON_LASSO_BOT").map(|v| v == "god").unwrap_or(false),
        };
        g.build_background();
        g
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.w = w;
        self.h = h;
        self.grid = Grid::new(w, h);
        self.build_background();
        let clamp = |p: &mut V2| {
            p.x = p.x.clamp(1.0, w - 1.0);
            p.y = p.y.clamp(1.0, h - 1.0);
        };
        clamp(&mut self.player.pos);
        self.player.trail.clear();
        self.player.trail_len = 0.0;
        for e in self.enemies.iter_mut() {
            clamp(&mut e.pos);
        }
    }

    fn build_background(&mut self) {
        let (w, h) = (self.w as usize, self.h as usize);
        let s1 = self.rng.f() * 1000.0;
        let hash = |x: i32, y: i32, s: f32| -> f32 {
            let v = ((x as f32 * 127.1 + y as f32 * 311.7 + s) .sin()) * 43758.547;
            v - v.floor()
        };
        let noise = |x: f32, y: f32, s: f32| -> f32 {
            let (xi, yi) = (x.floor() as i32, y.floor() as i32);
            let (fx, fy) = (x - x.floor(), y - y.floor());
            let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
            let a = hash(xi, yi, s);
            let b = hash(xi + 1, yi, s);
            let c = hash(xi, yi + 1, s);
            let d = hash(xi + 1, yi + 1, s);
            (a + (b - a) * ux) + ((c + (d - c) * ux) - (a + (b - a) * ux)) * uy
        };
        let fbm = |x: f32, y: f32, s: f32| -> f32 {
            let mut v = 0.0;
            let mut amp = 0.5;
            let mut f = 1.0;
            for _ in 0..5 {
                v += noise(x * f, y * f, s) * amp;
                f *= 2.03;
                amp *= 0.5;
            }
            v
        };
        self.nebula = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let fx = x as f32 / 38.0;
                let fy = y as f32 / 38.0;
                let n1 = fbm(fx, fy, s1);
                let n2 = fbm(fx * 1.7 + 5.2, fy * 1.7 - 1.3, s1 + 17.0);
                let gy = y as f32 / h as f32;
                let mut c = rgb(0.008, 0.010, 0.030).lerp(rgb(0.02, 0.006, 0.035), gy);
                let a = ((n1 - 0.45) * 2.6).clamp(0.0, 1.0);
                let b = ((n2 - 0.5) * 3.0).clamp(0.0, 1.0);
                c += rgb(0.075, 0.018, 0.12) * (a * a);
                c += rgb(0.0, 0.06, 0.08) * (b * b * b);
                self.nebula.push(c);
            }
        }
        let count = (w * h / 110).max(30);
        self.stars.clear();
        for _ in 0..count {
            let hue = self.rng.range(0.5, 0.75);
            let depth = self.rng.f().powf(2.0);
            self.stars.push(Star {
                pos: v2(self.rng.range(0.0, self.w), self.rng.range(0.0, self.h)),
                depth,
                phase: self.rng.angle(),
                col: hsv(hue, self.rng.range(0.0, 0.5), 1.0),
            });
        }
    }

    // ---------------------------------------------------------------- input

    pub fn press(&mut self, d: usize, repeat: bool) {
        if self.mode != Mode::Playing {
            return;
        }
        if self.held_mode {
            if !repeat {
                self.held[d] = true;
            }
            return;
        }
        // Without key-release events: a quick second perpendicular key makes a diagonal.
        let mut dir = DIRS[d];
        if let Some((pd, pt)) = self.last_press {
            if self.t - pt < 0.11 && pd / 2 != d / 2 {
                dir = (DIRS[pd] + DIRS[d]).norm();
            }
        }
        self.player.target = dir.angle();
        self.last_press = Some((d, self.t));
    }

    pub fn release(&mut self, d: usize) {
        self.held[d] = false;
    }

    pub fn dash(&mut self) {
        if self.mode != Mode::Playing || !self.player.alive {
            return;
        }
        if self.player.energy >= 0.33 && self.player.dash <= 0.0 {
            self.player.energy -= 0.33;
            self.player.dash = 0.28;
            let p = self.player.pos;
            self.rings.push(Ring { pos: p, r: 1.0, speed: 70.0, life: 0.3, max: 0.3, col: rgb(0.4, 0.9, 1.0) });
            self.grid.impulse(p, 22.0, 60.0);
            self.shake += 0.6;
        }
    }

    pub fn confirm(&mut self) {
        match self.mode {
            Mode::Title => self.start(),
            Mode::GameOver if self.gameover_t <= 0.0 => self.start(),
            Mode::Paused => self.mode = Mode::Playing,
            _ => {}
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.mode {
            Mode::Playing => self.mode = Mode::Paused,
            Mode::Paused => self.mode = Mode::Playing,
            _ => {}
        }
    }

    pub fn back(&mut self) {
        match self.mode {
            Mode::Playing => self.mode = Mode::Paused,
            Mode::Paused | Mode::GameOver => {
                self.mode = Mode::Title;
                self.player.alive = true;
                self.player.invuln = 0.0;
            }
            Mode::Title => self.quit = true,
        }
    }

    fn start(&mut self) {
        // Blow away whatever is on screen for a punchy start.
        let es: Vec<(V2, Col)> = self.enemies.iter().map(|e| (e.pos, e.kind.color())).collect();
        for (p, c) in es {
            self.explode(p, c, 18, false);
        }
        self.enemies.clear();
        self.bullets.clear();
        self.gems.clear();
        self.mode = Mode::Playing;
        self.gt = 0.0;
        self.score = 0;
        self.shown_score = 0.0;
        self.new_best = false;
        self.mult = 1;
        self.mult_timer = 0.0;
        self.lives = 3;
        self.kills = 0;
        self.best_loop = 0;
        self.loops_closed = 0;
        self.spawn_t = 2.5;
        self.swarm_t = 14.0;
        self.held = [false; 4];
        let center = v2(self.w / 2.0, self.h / 2.0);
        self.player = Player {
            pos: v2(self.w / 2.0, self.h * 0.82),
            ang: 0.0,
            target: 0.0,
            dash: 0.0,
            energy: 1.0,
            invuln: 1.5,
            alive: true,
            trail: VecDeque::new(),
            trail_len: 0.0,
        };
        self.flash = 0.35;
        self.flash_col = rgb(0.5, 0.8, 1.0);
        self.shake += 2.0;
        self.grid.impulse(center, 80.0, 120.0);
        let r = (self.h * 0.15).clamp(8.0, 18.0);
        self.spawn_swarm(center, r, 7, Kind::Drifter);
        self.popup(center + v2(0.0, -r - 10.0), "LOOP THEM!", rgb(0.6, 1.0, 1.0), 2.2, 1, true);
    }

    // ---------------------------------------------------------------- helpers

    fn popup(&mut self, pos: V2, text: &str, col: Col, life: f32, scale: i32, rainbow: bool) {
        let scale = scale.min(fit_scale(text, self.w * 0.9, scale));
        let half = crate::font::text_width(text) as f32 * scale as f32 / 2.0;
        let pos = v2(pos.x.clamp(half + 1.0, (self.w - half - 1.0).max(half + 1.0)), pos.y.clamp(6.0 * scale as f32, self.h - 4.0 * scale as f32));
        self.popups.push(Popup { pos, text: text.to_string(), col, life, max: life, scale, rainbow });
    }

    fn spark(&mut self, pos: V2, vel: V2, life: f32, col: Col, drag: f32, kind: PKind) {
        if self.particles.len() < 6000 {
            self.particles.push(Particle { pos, vel, life, max: life, col, drag, kind });
        }
    }

    fn explode(&mut self, p: V2, col: Col, n: usize, gems: bool) {
        for _ in 0..n {
            let d = self.rng.dir();
            let s = self.rng.range(15.0, 130.0);
            let life = self.rng.range(0.25, 0.9);
            let c = col.lerp(WHITE, self.rng.f() * 0.3) * self.rng.range(0.9, 1.8);
            self.spark(p, d * s, life, c, 3.5, PKind::Spark);
        }
        for _ in 0..n / 3 {
            let d = self.rng.dir();
            let s = self.rng.range(5.0, 40.0);
            let life = self.rng.range(0.5, 1.4);
            self.spark(p, d * s, life, col * 0.9, 2.0, PKind::Dot);
        }
        self.spark(p, V2::default(), 0.22, col.lerp(WHITE, 0.3) * 0.9, 0.0, PKind::Glow(7.0));
        self.rings.push(Ring { pos: p, r: 1.0, speed: 55.0, life: 0.4, max: 0.4, col: col * 1.4 });
        self.grid.impulse(p, 26.0, 90.0);
        if gems {
            let k = 3 + (self.rng.f() * 3.0) as usize;
            for _ in 0..k {
                let v = self.rng.dir() * self.rng.range(20.0, 55.0);
                self.gems.push(Gem { pos: p, vel: v, age: 0.0 });
            }
        }
    }

    fn spawn_pos(&mut self, margin: f32) -> V2 {
        let mut p = v2(self.w / 2.0, self.h / 2.0);
        for _ in 0..20 {
            p = v2(self.rng.range(margin, self.w - margin), self.rng.range(margin, self.h - margin));
            if (p - self.player.pos).len() > 32.0 {
                break;
            }
        }
        p
    }

    fn spawn_enemy(&mut self, kind: Kind, pos: V2) {
        let diff = 1.0 + self.gt / 160.0;
        let vel = match kind {
            Kind::Drifter => self.rng.dir() * self.rng.range(8.0, 18.0) * diff,
            Kind::Spinner => self.rng.dir() * self.rng.range(3.0, 7.0),
            _ => V2::default(),
        };
        let spin = self.rng.range(-3.0, 3.0);
        let ang = self.rng.angle();
        let fire = self.rng.range(1.0, 2.0);
        self.enemies.push(Enemy { kind, pos, vel, ang, spin, spawn: SPAWN_TIME, fire_t: fire, cut_cd: 0.0, age: 0.0 });
    }

    fn spawn_swarm(&mut self, c: V2, r: f32, n: usize, kind: Kind) {
        let off = self.rng.angle();
        let c = v2(c.x.clamp(r + 4.0, self.w - r - 4.0), c.y.clamp(r + 4.0, self.h - r - 4.0));
        for i in 0..n {
            let a = off + i as f32 / n as f32 * TAU;
            let p = c + V2::from_angle(a) * r;
            self.spawn_enemy(kind, p);
            if let Some(e) = self.enemies.last_mut() {
                e.vel = V2::from_angle(a + PI / 2.0) * 6.0;
                e.spawn = SPAWN_TIME + i as f32 * 0.04;
            }
        }
    }

    fn base_speed(&self) -> f32 {
        (self.w.min(self.h * 2.2) * 0.42).clamp(55.0, 95.0)
    }

    // ---------------------------------------------------------------- update

    pub fn update(&mut self, real_dt: f32) {
        self.t += real_dt;
        self.shake *= (-real_dt * 9.0).exp();
        self.flash *= (-real_dt * 7.0).exp();
        self.aberr *= (-real_dt * 6.0).exp();
        self.hearts_flash = (self.hearts_flash - real_dt).max(0.0);
        if self.mode == Mode::Paused {
            return;
        }
        if self.hitstop > 0.0 {
            self.hitstop -= real_dt;
            return;
        }
        let mut scale = 1.0;
        if self.slowmo > 0.0 {
            self.slowmo -= real_dt;
            scale = 0.3 + 0.7 * (1.0 - (self.slowmo / 0.7).clamp(0.0, 1.0)).powi(2);
        }
        let dt = real_dt * scale;

        if self.mode == Mode::Playing && self.player.alive {
            self.gt += dt;
        }

        self.update_player(dt);
        self.update_enemies(dt);
        self.update_bullets(dt);
        self.collisions();
        self.update_fx(dt);
        self.grid.update(dt);
        self.spawning(dt);

        // multiplier decay
        if self.mult > 1 {
            self.mult_timer -= dt;
            if self.mult_timer <= 0.0 {
                self.mult_decay -= dt;
                if self.mult_decay <= 0.0 {
                    self.mult -= 1;
                    self.mult_decay = 0.22;
                }
            }
        }

        let target = self.score as f64;
        self.shown_score += (target - self.shown_score) * (1.0 - (-real_dt as f64 * 9.0).exp());
        if (target - self.shown_score).abs() < 1.0 {
            self.shown_score = target;
        }

        if self.mode == Mode::Playing && self.lives == 0 && !self.player.alive {
            self.gameover_t -= real_dt;
            if self.gameover_t <= 0.0 {
                self.mode = Mode::GameOver;
                self.gameover_t = 0.6;
            }
        } else if self.mode == Mode::GameOver {
            self.gameover_t -= real_dt;
        }
    }

    fn update_player(&mut self, dt: f32) {
        if !self.player.alive {
            if self.mode == Mode::Playing && self.lives > 0 {
                self.respawn_t -= dt;
                if self.respawn_t <= 0.0 {
                    self.respawn();
                }
            }
            return;
        }
        let speed0 = self.base_speed();
        let autopilot = self.mode == Mode::Title || self.bot;
        if autopilot {
            self.player.target = self.bot_steer(dt);
        } else if self.held_mode {
            let mut d = V2::default();
            for i in 0..4 {
                if self.held[i] {
                    d += DIRS[i];
                }
            }
            if d.len2() > 0.01 {
                self.player.target = d.angle();
            }
        }

        let p = &mut self.player;
        let turn = if autopilot { 7.0 } else { 10.0 };
        let diff = wrap_angle(p.target - p.ang);
        let max = turn * dt;
        p.ang += diff.clamp(-max, max);
        let dashing = p.dash > 0.0;
        let speed = speed0 * if dashing { 2.7 } else { 1.0 };
        let fwd = V2::from_angle(p.ang);
        p.pos += fwd * speed * dt;
        p.dash -= dt;
        p.invuln -= dt;
        p.energy = (p.energy + dt * 0.11).min(1.0);

        // walls: bounce
        let mut bounced = None;
        if p.pos.x < 1.0 {
            p.pos.x = 1.0;
            p.ang = PI - p.ang;
            p.target = PI - p.target;
            bounced = Some(v2(1.0, 0.0));
        } else if p.pos.x > self.w - 1.0 {
            p.pos.x = self.w - 1.0;
            p.ang = PI - p.ang;
            p.target = PI - p.target;
            bounced = Some(v2(-1.0, 0.0));
        }
        if p.pos.y < 1.0 {
            p.pos.y = 1.0;
            p.ang = -p.ang;
            p.target = -p.target;
            bounced = Some(v2(0.0, 1.0));
        } else if p.pos.y > self.h - 1.0 {
            p.pos.y = self.h - 1.0;
            p.ang = -p.ang;
            p.target = -p.target;
            bounced = Some(v2(0.0, -1.0));
        }
        let pos = p.pos;
        if let Some(n) = bounced {
            for _ in 0..10 {
                let v = (n + self.rng.dir() * 0.8).norm() * self.rng.range(20.0, 70.0);
                let life = self.rng.range(0.2, 0.5);
                self.spark(pos, v, life, rgb(0.5, 0.9, 1.0) * 1.5, 4.0, PKind::Spark);
            }
            self.grid.impulse(pos, 14.0, 40.0);
        }

        // engine sparks
        let back = -V2::from_angle(self.player.ang);
        let n = if dashing { 4 } else { 1 };
        for _ in 0..n {
            let v = (back + self.rng.dir() * 0.5) * self.rng.range(8.0, 30.0);
            let c = if dashing { rgb(0.7, 0.9, 1.0) * 1.6 } else { self.trail_head_col() * 0.9 };
            let life = self.rng.range(0.15, 0.4);
            self.spark(pos, v, life, c, 3.0, PKind::Dot);
        }
        // gentle grid wake
        self.grid.impulse(pos, 10.0, if dashing { 30.0 } else { 6.0 } * dt * 60.0 * 0.1);

        // trail
        let p = &mut self.player;
        let push = match p.trail.back() {
            Some(last) => (pos - *last).len() >= 1.1,
            None => true,
        };
        if push {
            if let Some(last) = p.trail.back() {
                p.trail_len += (pos - *last).len();
            }
            p.trail.push_back(pos);
            self.check_loop();
        }
        let max_len = self.max_trail();
        let p = &mut self.player;
        while p.trail_len > max_len && p.trail.len() > 2 {
            let a = p.trail.pop_front().unwrap();
            let b = p.trail[0];
            p.trail_len -= (b - a).len();
        }
    }

    /// Lasso AI used for the attract mode (and NEON_LASSO_BOT=1).
    fn bot_steer(&mut self, dt: f32) -> f32 {
        let pos = self.player.pos;
        self.bot_t -= dt;
        if self.loops_closed != self.bot_loops || self.bot_t <= 0.0 {
            self.bot_loops = self.loops_closed;
            self.bot_center = None;
        }
        if self.bot_center.is_none() {
            // pick the densest cluster that is not right on top of us
            let mut best = None;
            let mut best_score = f32::MIN;
            for e in &self.enemies {
                let n = self.enemies.iter().filter(|o| (o.pos - e.pos).len() < 20.0).count() as f32;
                let d = (e.pos - pos).len();
                let sc = n * 10.0 - d * 0.15 + if d < 14.0 { -30.0 } else { 0.0 };
                if sc > best_score {
                    best_score = sc;
                    best = Some(e.pos);
                }
            }
            let c = best.unwrap_or(v2(self.w / 2.0, self.h / 2.0));
            self.bot_center = Some(c);
            self.bot_auto_dir();
            self.bot_t = 6.0;
        }
        let mut c = self.bot_center.unwrap();
        // follow the cluster as it drifts
        let mut sum = V2::default();
        let mut cnt = 0.0;
        let mut far: f32 = 0.0;
        for e in &self.enemies {
            if (e.pos - c).len() < 20.0 {
                sum += e.pos;
                cnt += 1.0;
            }
        }
        if cnt > 0.0 {
            c = c.lerp(sum * (1.0 / cnt), (dt * 3.0).min(1.0));
            for e in &self.enemies {
                if (e.pos - c).len() < 20.0 {
                    far = far.max((e.pos - c).len());
                }
            }
        }
        let m = 10.0;
        c.x = c.x.clamp(m, self.w - m);
        c.y = c.y.clamp(m, self.h - m);
        self.bot_center = Some(c);
        let target_r = (far + 8.0).clamp(10.0, 30.0);
        self.bot_r += (target_r - self.bot_r) * (dt * 2.0).min(1.0);
        let rel = pos - c;
        let d = rel.len().max(0.01);
        let radial = rel * (1.0 / d);
        let tangent = radial.perp() * self.auto_dir;
        let err = (self.bot_r - d) / self.bot_r;
        let mut desired = tangent + radial * (err * 2.5);
        // dodge anything right in front of us
        let fwd = V2::from_angle(self.player.ang);
        let mut dodge = |p: V2, r: f32| {
            let v = pos - p;
            let l = v.len();
            if l < r && (-v).norm().x * fwd.x + (-v).norm().y * fwd.y > -0.2 {
                desired += v.norm() * ((1.0 - l / r) * 4.0);
            }
        };
        for e in &self.enemies {
            dodge(e.pos, 9.0);
        }
        for b in &self.bullets {
            dodge(b.pos, 7.0);
        }
        // keep off the walls
        let wm = 6.0;
        if pos.x < wm {
            desired.x += 2.0;
        }
        if pos.x > self.w - wm {
            desired.x -= 2.0;
        }
        if pos.y < wm {
            desired.y += 2.0;
        }
        if pos.y > self.h - wm {
            desired.y -= 2.0;
        }
        desired.angle()
    }

    fn bot_auto_dir(&mut self) {
        self.auto_dir = if self.rng.chance(0.5) { 1.0 } else { -1.0 };
    }

    fn max_trail(&self) -> f32 {
        let base = (self.w + self.h) * 0.9;
        base.clamp(120.0, 300.0) + (self.mult.min(30) as f32) * 4.0
    }

    fn trail_head_col(&self) -> Col {
        let h = 0.5 - (self.mult.min(24) as f32) * 0.018;
        hsv(h, 0.55, 1.0)
    }

    fn check_loop(&mut self) {
        let tr = &self.player.trail;
        let n = tr.len();
        if n < 8 {
            return;
        }
        let a = tr[n - 2];
        let b = tr[n - 1];
        let mut found = None;
        for i in (0..n - 5).rev() {
            if let Some(x) = seg_intersect(a, b, tr[i], tr[i + 1]) {
                found = Some((i, x));
                break;
            }
        }
        let Some((i, x)) = found else { return };
        let mut poly = Vec::with_capacity(n - i);
        poly.push(x);
        for j in (i + 1)..(n - 1) {
            poly.push(tr[j]);
        }
        if poly.len() < 4 || poly_area(&poly) < 18.0 {
            return;
        }
        let head = tr[n - 1];
        self.player.trail.clear();
        self.player.trail.push_back(x);
        self.player.trail.push_back(head);
        self.player.trail_len = (head - x).len();
        self.trigger_loop(poly);
    }

    fn trigger_loop(&mut self, poly: Vec<V2>) {
        let mut center = V2::default();
        for p in &poly {
            center += *p;
        }
        center *= 1.0 / poly.len() as f32;

        let mut killed: Vec<(V2, Kind)> = vec![];
        self.enemies.retain(|e| {
            if e.spawn <= 0.0 && point_in_poly(e.pos, &poly) {
                killed.push((e.pos, e.kind));
                false
            } else {
                true
            }
        });
        let mut popped = vec![];
        self.bullets.retain(|b| {
            if point_in_poly(b.pos, &poly) {
                popped.push(b.pos);
                false
            } else {
                true
            }
        });
        let n = killed.len() as u32;
        for p in &popped {
            for _ in 0..5 {
                let v = self.rng.dir() * self.rng.range(10.0, 50.0);
                self.spark(*p, v, 0.3, rgb(1.0, 0.4, 0.5), 4.0, PKind::Spark);
            }
        }
        let hue = 0.5 - (n as f32) * 0.07;
        let lcol = if n == 0 { rgb(0.3, 0.6, 1.0) * 0.5 } else { hsv(hue, 0.6, 1.0) };
        self.loops.push(LoopFx { pts: poly, center, life: 0.5, max: 0.5, col: lcol });
        self.loops_closed += 1;
        if n == 0 {
            self.grid.impulse(center, 20.0, 30.0);
            if !popped.is_empty() && self.mode == Mode::Playing {
                self.score += popped.len() as u64 * 10 * self.mult as u64;
            }
            return;
        }

        let playing = self.mode == Mode::Playing;
        let base: u64 = killed.iter().map(|(_, k)| k.points()).sum::<u64>() + popped.len() as u64 * 10;
        let gain = base * n as u64 * self.mult as u64;
        for (p, k) in &killed {
            self.explode(*p, k.color(), 34, true);
            self.bolts.push(Bolt { a: center, b: *p, life: 0.18, col: k.color().lerp(WHITE, 0.4) });
        }
        if playing {
            self.score += gain;
            self.kills += n;
            self.best_loop = self.best_loop.max(n);
            self.mult = (self.mult + n).min(99);
            self.mult_timer = 3.5;
            self.mult_decay = 0.0;
            self.player.energy = (self.player.energy + 0.07 * n as f32).min(1.0);
            let big = n >= 3;
            self.popup(center, &format!("+{}", fmt_num(gain)), lcol.lerp(WHITE, 0.4), 1.1, if big && self.w > 110.0 { 2 } else { 1 }, false);
        }
        let label = match n {
            1 => None,
            2 => Some("DOUBLE!"),
            3 => Some("TRIPLE!"),
            4 => Some("QUAD!"),
            5..=7 => Some("SUPERNOVA!"),
            8..=11 => Some("HYPERNOVA!"),
            _ => Some("SINGULARITY!"),
        };
        if let Some(l) = label {
            if playing {
                let sc = if n >= 5 && self.w > 140.0 { 2 } else { 1 };
                self.popup(center + v2(0.0, -9.0 - 8.0 * sc as f32), l, lcol, 1.4, sc, n >= 5);
            }
        }
        self.hitstop = (0.035 + 0.014 * n as f32).min(0.16);
        self.shake += 1.2 + n as f32 * 0.7;
        self.flash = (self.flash + 0.05 * n as f32).min(0.5);
        self.flash_col = lcol;
        self.grid.impulse(center, 30.0 + n as f32 * 6.0, 120.0 + n as f32 * 25.0);
        self.rings.push(Ring { pos: center, r: 2.0, speed: 90.0 + n as f32 * 10.0, life: 0.55, max: 0.55, col: lcol * 1.5 });
        if n >= 5 {
            self.slowmo = 0.7;
            self.aberr = 2.5;
            self.rings.push(Ring { pos: center, r: 1.0, speed: 160.0, life: 0.8, max: 0.8, col: WHITE * 1.2 });
            for _ in 0..80 {
                let v = self.rng.dir() * self.rng.range(40.0, 200.0);
                let c = hsv(self.rng.f(), 0.7, 1.0) * 1.6;
                let life = self.rng.range(0.4, 1.1);
                self.spark(center, v, life, c, 2.5, PKind::Spark);
            }
        }
    }

    fn respawn(&mut self) {
        let c = v2(self.w / 2.0, self.h / 2.0);
        let p = &mut self.player;
        p.pos = c;
        p.ang = -PI / 2.0;
        p.target = -PI / 2.0;
        p.alive = true;
        p.invuln = 2.5;
        p.dash = 0.0;
        p.energy = 1.0;
        p.trail.clear();
        p.trail_len = 0.0;
        self.held = [false; 4];
        let near: Vec<(V2, Col)> = self.enemies.iter().filter(|e| (e.pos - c).len() < 30.0).map(|e| (e.pos, e.kind.color())).collect();
        self.enemies.retain(|e| (e.pos - c).len() >= 30.0);
        for (p, col) in near {
            self.explode(p, col, 16, false);
        }
        self.rings.push(Ring { pos: c, r: 30.0, speed: -60.0, life: 0.45, max: 0.45, col: rgb(0.5, 1.0, 1.0) * 1.5 });
        self.grid.impulse(c, 40.0, -80.0);
    }

    fn kill_player(&mut self) {
        let pos = self.player.pos;
        self.player.alive = false;
        self.lives = self.lives.saturating_sub(1);
        self.hearts_flash = 1.0;
        for _ in 0..260 {
            let v = self.rng.dir() * self.rng.range(10.0, 220.0);
            let c = rgb(0.5, 1.0, 1.0).lerp(WHITE, self.rng.f()) * self.rng.range(1.0, 2.2);
            let life = self.rng.range(0.4, 1.6);
            self.spark(pos, v, life, c, 1.8, PKind::Spark);
        }
        // shatter the trail into motes
        let pts: Vec<V2> = self.player.trail.iter().copied().collect();
        for (i, p) in pts.iter().enumerate() {
            if i % 2 == 0 {
                let v = self.rng.dir() * self.rng.range(3.0, 20.0);
                let life = self.rng.range(0.5, 1.4);
                self.spark(*p, v, life, rgb(0.5, 0.3, 1.0), 1.0, PKind::Dot);
            }
        }
        self.player.trail.clear();
        self.player.trail_len = 0.0;
        self.spark(pos, V2::default(), 0.4, WHITE * 3.0, 0.0, PKind::Glow(22.0));
        for (i, s) in [60.0, 110.0, 170.0].iter().enumerate() {
            self.rings.push(Ring { pos, r: 1.0, speed: *s, life: 0.6 + i as f32 * 0.2, max: 0.6 + i as f32 * 0.2, col: rgb(0.5, 0.9, 1.0) * 1.5 });
        }
        self.grid.impulse(pos, 70.0, 260.0);
        self.shake += 7.0;
        self.flash = 0.7;
        self.flash_col = rgb(1.0, 0.3, 0.4);
        self.aberr = 4.0;
        self.slowmo = 0.9;
        self.mult = 1;
        let bl: Vec<V2> = self.bullets.iter().map(|b| b.pos).collect();
        for b in bl {
            self.spark(b, V2::default(), 0.3, rgb(1.0, 0.3, 0.4), 0.0, PKind::Glow(3.0));
        }
        self.bullets.clear();
        if self.lives == 0 {
            self.gameover_t = 1.6;
            if self.score > self.best {
                self.best = self.score;
                self.new_best = true;
                if let Some(p) = best_path() {
                    let _ = std::fs::write(p, self.best.to_string());
                }
            }
        } else {
            self.respawn_t = 1.3;
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let ppos = self.player.pos;
        let alive = self.player.alive;
        let diff = 1.0 + self.gt / 160.0;
        let (w, h) = (self.w, self.h);
        let mut cuts: Vec<(usize, V2)> = vec![];
        for i in 0..self.enemies.len() {
            let e = &mut self.enemies[i];
            e.age += dt;
            e.ang += e.spin * dt;
            if e.spawn > 0.0 {
                e.spawn -= dt;
                continue;
            }
            match e.kind {
                Kind::Drifter => {}
                Kind::Seeker => {
                    if alive {
                        let d = (ppos - e.pos).norm();
                        e.vel += d * 70.0 * dt;
                    }
                    let max = 30.0 * diff;
                    if e.vel.len() > max {
                        e.vel = e.vel.norm() * max;
                    }
                    e.ang = e.vel.angle();
                }
                Kind::Snipper => {
                    e.cut_cd -= dt;
                    let tr = &self.player.trail;
                    let mut target = ppos;
                    let mut best = f32::MAX;
                    let mut j = 0;
                    while j < tr.len() {
                        let d = (tr[j] - e.pos).len2();
                        if d < best {
                            best = d;
                            target = tr[j];
                        }
                        j += 3;
                    }
                    let d = (target - e.pos).norm();
                    e.vel += d * 60.0 * dt;
                    let max = 24.0 * diff;
                    if e.vel.len() > max {
                        e.vel = e.vel.norm() * max;
                    }
                    if e.cut_cd <= 0.0 && tr.len() > 2 {
                        for k in 0..tr.len() - 1 {
                            if dist_to_seg(e.pos, tr[k], tr[k + 1]) < 2.6 {
                                cuts.push((k + 1, e.pos));
                                e.cut_cd = 0.6;
                                break;
                            }
                        }
                    }
                }
                Kind::Spinner => {
                    e.fire_t -= dt;
                    if e.fire_t <= 0.0 {
                        e.fire_t = (1.5 / diff).max(0.7);
                        let n = if self.gt > 120.0 { 6 } else { 4 };
                        let base = e.ang;
                        let spd = 26.0 * diff.min(1.8);
                        for k in 0..n {
                            let a = base + k as f32 / n as f32 * TAU;
                            self.bullets.push(Bullet { pos: e.pos, vel: V2::from_angle(a) * spd, life: 6.0 });
                        }
                    }
                }
            }
            let e = &mut self.enemies[i];
            e.pos += e.vel * dt;
            let r = e.kind.radius();
            if e.pos.x < r {
                e.pos.x = r;
                e.vel.x = e.vel.x.abs();
            }
            if e.pos.x > w - r {
                e.pos.x = w - r;
                e.vel.x = -e.vel.x.abs();
            }
            if e.pos.y < r {
                e.pos.y = r;
                e.vel.y = e.vel.y.abs();
            }
            if e.pos.y > h - r {
                e.pos.y = h - r;
                e.vel.y = -e.vel.y.abs();
            }
        }
        // soft separation so swarms don't stack
        let n = self.enemies.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.enemies[j].pos - self.enemies[i].pos;
                let rr = self.enemies[i].kind.radius() + self.enemies[j].kind.radius();
                let l2 = d.len2();
                if l2 < rr * rr && l2 > 1e-4 {
                    let l = l2.sqrt();
                    let push = d * ((rr - l) / l * 0.5 * (dt * 8.0).min(1.0));
                    self.enemies[i].pos -= push;
                    self.enemies[j].pos += push;
                }
            }
        }
        for (k, pos) in cuts {
            let tr = &mut self.player.trail;
            let k = k.min(tr.len());
            let removed: Vec<V2> = tr.drain(..k).collect();
            let mut len = 0.0;
            for w in 0..tr.len().saturating_sub(1) {
                len += (tr[w + 1] - tr[w]).len();
            }
            self.player.trail_len = len;
            for (q, p) in removed.iter().enumerate() {
                if q % 2 == 0 {
                    let v = self.rng.dir() * self.rng.range(4.0, 18.0);
                    let life = self.rng.range(0.3, 0.8);
                    self.spark(*p, v, life, rgb(1.0, 0.3, 0.3), 2.0, PKind::Dot);
                }
            }
            for _ in 0..14 {
                let v = self.rng.dir() * self.rng.range(20.0, 80.0);
                self.spark(pos, v, 0.35, rgb(1.0, 0.5, 0.3) * 1.5, 4.0, PKind::Spark);
            }
            self.rings.push(Ring { pos, r: 1.0, speed: 40.0, life: 0.25, max: 0.25, col: rgb(1.0, 0.3, 0.2) });
            self.shake += 0.6;
        }
    }

    fn update_bullets(&mut self, dt: f32) {
        let (w, h) = (self.w, self.h);
        for b in self.bullets.iter_mut() {
            b.pos += b.vel * dt;
            b.life -= dt;
        }
        self.bullets.retain(|b| b.life > 0.0 && b.pos.x > -2.0 && b.pos.y > -2.0 && b.pos.x < w + 2.0 && b.pos.y < h + 2.0);
    }

    fn collisions(&mut self) {
        if self.mode != Mode::Playing || !self.player.alive {
            return;
        }
        let p = &self.player;
        if p.invuln > 0.0 || p.dash > 0.0 || self.god {
            return;
        }
        let pos = p.pos;
        let hit_e = self.enemies.iter().any(|e| e.spawn <= 0.0 && (e.pos - pos).len() < e.kind.radius() * 0.75 + 1.0);
        let hit_b = self.bullets.iter().any(|b| (b.pos - pos).len() < 1.9);
        if hit_e || hit_b {
            self.kill_player();
        }
    }

    fn update_fx(&mut self, dt: f32) {
        for p in self.particles.iter_mut() {
            p.pos += p.vel * dt;
            p.vel *= (-p.drag * dt).exp();
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        let ppos = self.player.pos;
        let alive = self.player.alive;
        let mut collected = 0;
        for g in self.gems.iter_mut() {
            g.age += dt;
            if alive && g.age > 0.35 {
                let d = ppos - g.pos;
                let l = d.len().max(0.5);
                g.vel += d * (1.0 / l) * (380.0 + 3000.0 / (l + 5.0)) * dt;
                g.vel *= (-4.0 * dt).exp();
                if l < 3.0 {
                    g.age = 99.0;
                    collected += 1;
                }
            } else {
                g.vel *= (-2.5 * dt).exp();
            }
            g.pos += g.vel * dt;
        }
        self.gems.retain(|g| g.age < 5.0);
        if collected > 0 {
            if self.mode == Mode::Playing {
                self.score += collected as u64 * 5 * self.mult as u64;
                self.player.energy = (self.player.energy + 0.012 * collected as f32).min(1.0);
            }
            for _ in 0..collected {
                let v = self.rng.dir() * self.rng.range(10.0, 35.0);
                self.spark(ppos, v, 0.25, rgb(1.0, 0.85, 0.3) * 1.5, 4.0, PKind::Dot);
            }
        }

        for r in self.rings.iter_mut() {
            r.r += r.speed * dt;
            r.life -= dt;
        }
        self.rings.retain(|r| r.life > 0.0 && r.r > 0.0);
        for p in self.popups.iter_mut() {
            p.life -= dt;
            p.pos.y -= 7.0 * dt;
        }
        self.popups.retain(|p| p.life > 0.0);
        for l in self.loops.iter_mut() {
            l.life -= dt;
        }
        self.loops.retain(|l| l.life > 0.0);
        for b in self.bolts.iter_mut() {
            b.life -= dt;
        }
        self.bolts.retain(|b| b.life > 0.0);
        for s in self.stars.iter_mut() {
            s.pos.x -= s.depth * 2.5 * dt;
            if s.pos.x < 0.0 {
                s.pos.x += self.w;
            }
        }
    }

    fn spawning(&mut self, dt: f32) {
        match self.mode {
            Mode::Title => {
                let want = ((self.w * self.h) / 900.0).clamp(6.0, 16.0) as usize;
                if self.enemies.len() < want {
                    self.spawn_t -= dt;
                    if self.spawn_t <= 0.0 {
                        let p = self.spawn_pos(6.0);
                        let k = if self.rng.chance(0.75) { Kind::Drifter } else { Kind::Spinner };
                        self.spawn_enemy(k, p);
                        self.spawn_t = 0.4;
                    }
                }
                self.bullets.clear();
            }
            Mode::Playing if self.player.alive => {
                let gt = self.gt;
                let area_cap = ((self.w * self.h) / 1000.0).clamp(6.0, 26.0);
                let cap = (area_cap * (1.0 + gt / 90.0).min(1.9)) as usize;
                self.spawn_t -= dt;
                if self.spawn_t <= 0.0 && self.enemies.len() < cap {
                    self.spawn_t = (1.35 - gt / 200.0).max(0.38);
                    let mut weights = vec![(Kind::Drifter, 1.0)];
                    if gt > 8.0 {
                        weights.push((Kind::Seeker, 0.55 + gt / 300.0));
                    }
                    if gt > 30.0 {
                        weights.push((Kind::Snipper, 0.3 + gt / 400.0));
                    }
                    if gt > 50.0 {
                        weights.push((Kind::Spinner, 0.25 + gt / 500.0));
                    }
                    let total: f32 = weights.iter().map(|w| w.1).sum();
                    let mut r = self.rng.f() * total;
                    let mut kind = Kind::Drifter;
                    for (k, wgt) in weights {
                        if r < wgt {
                            kind = k;
                            break;
                        }
                        r -= wgt;
                    }
                    let p = self.spawn_pos(6.0);
                    self.spawn_enemy(kind, p);
                }
                self.swarm_t -= dt;
                if self.swarm_t <= 0.0 {
                    self.swarm_t = (17.0 - gt / 30.0).max(9.0);
                    let c = self.spawn_pos(20.0);
                    let r = (self.h * 0.14).clamp(8.0, 18.0);
                    let n = (6.0 + gt / 25.0).min(12.0) as usize;
                    let kind = if gt > 70.0 && self.rng.chance(0.4) { Kind::Seeker } else { Kind::Drifter };
                    self.spawn_swarm(c, r, n, kind);
                    self.popup(c, "SWARM", rgb(0.5, 1.0, 0.7), 1.2, 1, false);
                }
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------- render

    pub fn render(&mut self, cv: &mut Canvas) {
        cv.clear_from(&self.nebula);
        cv.cam = V2::default();

        // stars (no shake: they're far away)
        for s in &self.stars {
            let tw = 0.55 + 0.45 * (self.t * (1.0 + s.depth * 2.0) + s.phase).sin();
            cv.splat(s.pos, s.col * ((0.08 + 0.55 * s.depth) * tw));
        }

        let shake = self.shake.min(8.0);
        cv.cam = self.rng.dir() * (shake * self.rng.f());

        let light = if self.player.alive { self.player.pos } else { v2(-999.0, -999.0) };
        self.grid.render(cv, light, self.trail_head_col());

        // closed-loop flashes: filled polygon collapsing toward its centre
        for l in &self.loops {
            let f = l.life / l.max;
            let shrink = (1.0 - f).powf(2.0);
            let pts: Vec<V2> = l.pts.iter().map(|p| p.lerp(l.center, shrink)).collect();
            cv.poly_fill(&pts, l.col * (0.2 * f * f));
            let n = pts.len();
            for i in 0..n {
                cv.line(pts[i], pts[(i + 1) % n], l.col * (1.6 * f), l.col * (1.6 * f));
            }
        }

        // trail
        let tr = &self.player.trail;
        let n = tr.len();
        if n >= 2 {
            let head = self.trail_head_col();
            let tail = rgb(0.45, 0.05, 0.9);
            let pulse = 1.0 + 0.15 * (self.t * 12.0).sin();
            for i in 0..n - 1 {
                let t0 = i as f32 / (n - 1) as f32;
                let t1 = (i + 1) as f32 / (n - 1) as f32;
                let c0 = tail.lerp(head, t0.powf(1.4)) * (0.18 + 1.1 * t0 * pulse);
                let c1 = tail.lerp(head, t1.powf(1.4)) * (0.18 + 1.1 * t1 * pulse);
                cv.line(tr[i], tr[i + 1], c0, c1);
            }
        }

        // bolts
        for b in &self.bolts {
            let d = b.b - b.a;
            let segs = ((d.len() / 4.0) as usize).max(2);
            let perp = d.perp().norm();
            let mut prev = b.a;
            for s in 1..=segs {
                let t = s as f32 / segs as f32;
                let j = if s == segs { 0.0 } else { self.rng.range(-2.0, 2.0) };
                let p = b.a + d * t + perp * j;
                cv.line(prev, p, b.col * 1.3, b.col * 1.3);
                prev = p;
            }
        }

        // gems
        for g in &self.gems {
            let f = if g.age > 4.0 { 5.0 - g.age } else { 1.0 };
            let tw = 0.7 + 0.3 * (self.t * 20.0 + g.pos.x).sin();
            cv.splat(g.pos, rgb(1.0, 0.85, 0.3) * (2.0 * f * tw));
            cv.glow(g.pos, 1.8, rgb(1.0, 0.6, 0.1) * (0.35 * f));
        }

        // enemies
        for e in &self.enemies {
            let col = e.kind.color();
            let r = e.kind.radius();
            if e.spawn > 0.0 {
                let p = 1.0 - (e.spawn / SPAWN_TIME).clamp(0.0, 1.0);
                cv.ring(e.pos, r + (1.0 - p) * 12.0, col * (0.9 * p));
                if (self.t * 30.0) as i32 % 2 == 0 {
                    self.draw_enemy(cv, e, col * (0.35 * p), r * p);
                }
                continue;
            }
            self.draw_enemy(cv, e, col, r);
        }

        // bullets
        for b in &self.bullets {
            let pulse = 0.8 + 0.2 * (self.t * 18.0 + b.pos.x).sin();
            cv.glow(b.pos, 2.8, rgb(1.0, 0.15, 0.3) * (0.9 * pulse));
            cv.splat(b.pos, rgb(1.0, 0.85, 0.9) * 2.4);
            cv.line(b.pos - b.vel * 0.06, b.pos, rgb(1.0, 0.2, 0.3) * 0.1, rgb(1.0, 0.3, 0.4) * 0.8);
        }

        // player
        if self.player.alive {
            let p = &self.player;
            let blink = p.invuln > 0.0 && (self.t * 14.0) as i32 % 2 == 0;
            let hc = self.trail_head_col();
            let fwd = V2::from_angle(p.ang);
            if !blink {
                let dashing = p.dash > 0.0;
                let gr = if dashing { 5.0 } else { 3.2 };
                cv.glow(p.pos, gr, hc * if dashing { 1.4 } else { 0.9 });
                cv.splat(p.pos, WHITE * 3.0);
                cv.splat(p.pos + fwd * 1.0, WHITE * 1.2);
                cv.line(p.pos - fwd.perp() * 1.6 - fwd * 1.2, p.pos + fwd * 1.8, hc * 0.8, WHITE);
                cv.line(p.pos + fwd.perp() * 1.6 - fwd * 1.2, p.pos + fwd * 1.8, hc * 0.8, WHITE);
            }
            if p.invuln > 0.0 {
                cv.ring(p.pos, 5.0 + (self.t * 6.0).sin(), hc * 0.4);
            }
        }

        // particles
        for p in &self.particles {
            let f = (p.life / p.max).clamp(0.0, 1.0);
            let c = p.col.lerp(WHITE, f.powi(4) * 0.8) * f;
            match p.kind {
                PKind::Spark => {
                    let tail = p.pos - p.vel * 0.035;
                    cv.line(tail, p.pos, c * 0.2, c);
                }
                PKind::Dot => cv.splat(p.pos, c),
                PKind::Glow(r) => cv.glow(p.pos, r * (1.5 - f * 0.5), c * 0.8),
            }
        }

        // shockwaves
        for r in &self.rings {
            let f = (r.life / r.max).clamp(0.0, 1.0);
            cv.ring(r.pos, r.r, r.col * (f * f));
            if r.r > 3.0 {
                cv.ring(r.pos, r.r * 0.85, r.col * (f * f * 0.3));
            }
        }

        // popups
        for p in &self.popups {
            let f = (p.life / p.max).clamp(0.0, 1.0);
            let a = if f < 0.3 { f / 0.3 } else { 1.0 };
            let pop = 1.0 + ((1.0 - f) * 30.0).min(1.0).mul_add(-1.0, 1.0) * 0.6;
            let t = self.t;
            if p.rainbow {
                cv.text_fx(&p.text, p.pos, p.scale, |ci, row| hsv(t * 1.5 + ci as f32 * 0.08 + row as f32 * 0.02, 0.7, 1.0) * (1.4 * a * pop));
            } else {
                let c = p.col * (1.3 * a * pop);
                cv.text(&p.text, p.pos, p.scale, c);
            }
        }

        cv.cam = V2::default();
        self.render_overlay(cv);

        cv.flash = self.flash_col * (self.flash * 0.6);
        cv.aberration = self.aberr;
    }

    fn draw_enemy(&self, cv: &mut Canvas, e: &Enemy, col: Col, r: f32) {
        if r <= 0.3 {
            return;
        }
        let pulse = 1.0 + 0.2 * (self.t * 6.0 + e.age).sin();
        match e.kind {
            Kind::Drifter => {
                cv.poly_outline(e.pos, r, 4, e.ang, col * 1.2);
                cv.splat(e.pos, col.lerp(WHITE, 0.5) * (1.0 * pulse));
            }
            Kind::Seeker => {
                let a = e.ang;
                let tip = e.pos + V2::from_angle(a) * r * 1.2;
                let l = e.pos + V2::from_angle(a + 2.5) * r;
                let rr = e.pos + V2::from_angle(a - 2.5) * r;
                cv.line(tip, l, col * 1.3, col);
                cv.line(l, rr, col, col);
                cv.line(rr, tip, col, col * 1.3);
                cv.splat(e.pos, col.lerp(WHITE, 0.4) * pulse);
            }
            Kind::Snipper => {
                let a = self.t * 9.0 + e.age;
                for k in 0..2 {
                    let d = V2::from_angle(a + k as f32 * PI / 2.0) * r;
                    cv.line(e.pos - d, e.pos + d, col * 1.4, col * 1.4);
                }
                cv.ring(e.pos, r * 0.6, col * 0.5);
                cv.splat(e.pos, WHITE * 0.8);
            }
            Kind::Spinner => {
                cv.poly_outline(e.pos, r, 6, e.ang * 0.4, col * 1.1);
                cv.poly_outline(e.pos, r * 0.5, 3, -e.ang, col.lerp(WHITE, 0.3) * 1.2);
                let charge = 1.0 - (e.fire_t / 1.5).clamp(0.0, 1.0);
                cv.glow(e.pos, 1.5 + charge * 1.5, col.lerp(WHITE, charge) * (0.6 + charge));
            }
        }
    }

    fn render_overlay(&mut self, cv: &mut Canvas) {
        let (w, h) = (self.w, self.h);
        let t = self.t;
        let cols = cv.cols as i32;
        let rows = cv.rows as i32;
        match self.mode {
            Mode::Title => {
                let words = ["NEON", "LASSO"];
                let scale = ((w * 0.8) / 34.0).floor().clamp(1.0, 5.0) as i32;
                let scale = scale.min(((h * 0.22) / 7.0).floor().max(1.0) as i32);
                let lh = 9.0 * scale as f32;
                let top = h * 0.34 - lh * 0.5;
                for (wi, word) in words.iter().enumerate() {
                    let c = v2(w / 2.0, top + wi as f32 * lh);
                    let wi = wi as f32;
                    cv.text_fx(word, c, scale, |ci, row| {
                        let hue = 0.52 + 0.12 * (t * 0.8 + ci as f32 * 0.35 + wi * 0.9).sin() + row as f32 * 0.012;
                        hsv(hue, 0.8, 1.0) * (1.0 + 0.2 * (t * 3.0 + ci as f32).sin())
                    });
                }
                let r0 = ((top + lh * 2.0) / 2.0) as i32 + 1;
                cv.put_text_centered(r0, "Close loops with your light trail to shatter everything inside", rgb(0.7, 0.85, 1.0));
                cv.put_text_centered(r0 + 2, "ARROWS / WASD  steer      SPACE  dash      P  pause      Q  quit", rgb(0.55, 0.6, 0.8));
                if (t * 2.0) as i32 % 2 == 0 {
                    cv.put_text_centered(r0 + 4, "▶  PRESS ENTER  ◀", rgb(1.0, 0.95, 0.6));
                }
                if self.best > 0 {
                    cv.put_text_centered(rows - 2, &format!("BEST  {}", fmt_num(self.best)), rgb(1.0, 0.8, 0.35));
                }
                if !self.held_mode {
                    cv.put_text_centered(rows - 1, "tip: tap two arrows quickly for diagonals", rgb(0.35, 0.4, 0.55));
                }
            }
            Mode::Playing | Mode::Paused | Mode::GameOver => {
                self.render_hud(cv);
                if self.mode == Mode::Paused {
                    let s = fit_scale("PAUSED", w * 0.6, 2);
                    cv.text("PAUSED", v2(w / 2.0, h / 2.0 - 4.0), s, rgb(0.6, 0.9, 1.0) * 1.3);
                    cv.put_text_centered(rows / 2 + 3, "ENTER / P  resume      ESC  menu", rgb(0.7, 0.75, 0.9));
                }
                if self.mode == Mode::GameOver {
                    let s = fit_scale("GAME OVER", w * 0.9, 3);
                    let cy = h * 0.36;
                    cv.text_fx("GAME OVER", v2(w / 2.0, cy), s, |ci, row| {
                        hsv(0.92 + 0.05 * (t * 2.0 + ci as f32 * 0.5).sin() + row as f32 * 0.01, 0.75, 1.0) * 1.4
                    });
                    let sy = cy + 7.0 * s as f32 + 8.0;
                    let ss = fit_scale(&fmt_num(self.score), w * 0.6, 2);
                    cv.text(&fmt_num(self.score), v2(w / 2.0, sy), ss, rgb(0.6, 1.0, 1.0) * 1.3);
                    let r0 = ((sy + 7.0 * ss as f32) / 2.0) as i32 + 2;
                    if self.new_best {
                        if (t * 4.0) as i32 % 2 == 0 {
                            cv.put_text_centered(r0, "★  NEW BEST  ★", rgb(1.0, 0.85, 0.3));
                        }
                    } else {
                        cv.put_text_centered(r0, &format!("best {}", fmt_num(self.best)), rgb(1.0, 0.8, 0.4));
                    }
                    cv.put_text_centered(
                        r0 + 2,
                        &format!("{} kills   ·   {} loops   ·   biggest loop {}   ·   {:.0}s", self.kills, self.loops_closed, self.best_loop, self.gt),
                        rgb(0.65, 0.7, 0.9),
                    );
                    if self.gameover_t <= 0.0 {
                        cv.put_text_centered(r0 + 4, "ENTER  play again      ESC  menu      Q  quit", rgb(0.9, 0.95, 1.0));
                    }
                }
                let _ = cols;
            }
        }
    }

    fn render_hud(&self, cv: &mut Canvas) {
        let cols = cv.cols as i32;
        let score = format!("SCORE {}", fmt_num(self.shown_score.round() as u64));
        cv.put_text(1, 0, &score, rgb(0.85, 1.0, 1.0));

        // multiplier + timer bar
        let mc = self.trail_head_col();
        let mtxt = format!("x{}", self.mult);
        let bar_w = 10;
        let frac = if self.mult > 1 { (self.mult_timer / 3.5).clamp(0.0, 1.0) } else { 0.0 };
        let filled = (frac * bar_w as f32).round() as usize;
        let bar: String = (0..bar_w).map(|i| if i < filled { '━' } else { '─' }).collect();
        let mx = cols / 2 - 7;
        let pulse = if self.mult > 1 { 0.75 + 0.25 * (self.t * 8.0).sin() } else { 0.6 };
        cv.put_text(mx, 0, &mtxt, mc * (0.6 + 0.6 * pulse));
        cv.put_text(mx + mtxt.len() as i32 + 1, 0, &bar, mc * 0.8);

        // dash energy + lives on the right
        let e = self.player.energy;
        let segs = 3;
        let mut dash = String::from("DASH ");
        for i in 0..segs {
            let lo = i as f32 / segs as f32;
            dash.push(if e >= lo + 0.33 { '█' } else if e > lo + 0.12 { '▓' } else { '░' });
        }
        let hearts: String = (0..3).map(|i| if i < self.lives { '♥' } else { '·' }).collect::<Vec<_>>().iter().map(|c| format!("{c} ")).collect();
        let right = format!("{dash}   {hearts}");
        let rx = cols - right.chars().count() as i32 - 1;
        let ec = if e >= 0.33 { rgb(0.5, 0.9, 1.0) } else { rgb(0.4, 0.45, 0.6) };
        cv.put_text(rx, 0, &dash, ec);
        let hc = if self.hearts_flash > 0.0 && (self.t * 12.0) as i32 % 2 == 0 { WHITE } else { rgb(1.0, 0.3, 0.5) };
        cv.put_text(rx + dash.chars().count() as i32 + 3, 0, &hearts, hc);
    }
}
