use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, s: f32) -> V2 {
        v2(self.x * s, self.y * s)
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        self.x += o.x;
        self.y += o.y;
    }
}
impl SubAssign for V2 {
    fn sub_assign(&mut self, o: V2) {
        self.x -= o.x;
        self.y -= o.y;
    }
}
impl MulAssign<f32> for V2 {
    fn mul_assign(&mut self, s: f32) {
        self.x *= s;
        self.y *= s;
    }
}

impl V2 {
    pub fn len2(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
    pub fn len(self) -> f32 {
        self.len2().sqrt()
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 1e-6 { self * (1.0 / l) } else { v2(0.0, 0.0) }
    }
    pub fn from_angle(a: f32) -> V2 {
        v2(a.cos(), a.sin())
    }
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }
    pub fn lerp(self, o: V2, t: f32) -> V2 {
        self + (o - self) * t
    }
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Col {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

pub const fn rgb(r: f32, g: f32, b: f32) -> Col {
    Col { r, g, b }
}

impl Add for Col {
    type Output = Col;
    fn add(self, o: Col) -> Col {
        rgb(self.r + o.r, self.g + o.g, self.b + o.b)
    }
}
impl AddAssign for Col {
    fn add_assign(&mut self, o: Col) {
        self.r += o.r;
        self.g += o.g;
        self.b += o.b;
    }
}
impl Mul<f32> for Col {
    type Output = Col;
    fn mul(self, s: f32) -> Col {
        rgb(self.r * s, self.g * s, self.b * s)
    }
}
impl Mul<Col> for Col {
    type Output = Col;
    fn mul(self, o: Col) -> Col {
        rgb(self.r * o.r, self.g * o.g, self.b * o.b)
    }
}

impl Col {
    pub fn lerp(self, o: Col, t: f32) -> Col {
        rgb(
            self.r + (o.r - self.r) * t,
            self.g + (o.g - self.g) * t,
            self.b + (o.b - self.b) * t,
        )
    }
}

pub const WHITE: Col = rgb(1.0, 1.0, 1.0);

/// h in [0,1), s,v in [0,1]
pub fn hsv(h: f32, s: f32, v: f32) -> Col {
    let h = (h.rem_euclid(1.0)) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i {
        0 => rgb(v, t, p),
        1 => rgb(q, v, p),
        2 => rgb(p, v, t),
        3 => rgb(p, q, v),
        4 => rgb(t, p, v),
        _ => rgb(v, p, q),
    }
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// [0,1)
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    pub fn angle(&mut self) -> f32 {
        self.f() * std::f32::consts::TAU
    }
    pub fn dir(&mut self) -> V2 {
        V2::from_angle(self.angle())
    }
}

pub fn wrap_angle(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    let mut a = a.rem_euclid(t);
    if a > std::f32::consts::PI {
        a -= t;
    }
    a
}

/// Segment intersection: returns the point where segment ab crosses segment cd.
pub fn seg_intersect(a: V2, b: V2, c: V2, d: V2) -> Option<V2> {
    let r = b - a;
    let s = d - c;
    let denom = r.x * s.y - r.y * s.x;
    if denom.abs() < 1e-8 {
        return None;
    }
    let ca = c - a;
    let t = (ca.x * s.y - ca.y * s.x) / denom;
    let u = (ca.x * r.y - ca.y * r.x) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(a + r * t)
    } else {
        None
    }
}

pub fn poly_area(p: &[V2]) -> f32 {
    let mut s = 0.0;
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        s += a.x * b.y - b.x * a.y;
    }
    (s * 0.5).abs()
}

pub fn point_in_poly(pt: V2, poly: &[V2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > pt.y) != (b.y > pt.y) {
            let x = a.x + (pt.y - a.y) * (b.x - a.x) / (b.y - a.y);
            if pt.x < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

pub fn dist_to_seg(p: V2, a: V2, b: V2) -> f32 {
    let ab = b - a;
    let l2 = ab.len2();
    if l2 < 1e-6 {
        return (p - a).len();
    }
    let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / l2).clamp(0.0, 1.0);
    (p - (a + ab * t)).len()
}
