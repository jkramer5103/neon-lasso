//! HDR additive canvas rendered to the terminal with half-block characters.
//! Each terminal cell holds two vertically stacked "pixels" (fg = top, bg = bottom).

use crate::font;
use crate::math::*;
use std::io::Write;

#[derive(Clone, Copy, PartialEq)]
struct Cell {
    top: [u8; 3],
    bot: [u8; 3],
    fg: [u8; 3],
    ch: char,
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub cols: usize,
    pub rows: usize,
    pub buf: Vec<Col>,
    pub cam: V2,
    pub aberration: f32,
    pub flash: Col,
    pub exposure: f32,
    comp: Vec<Col>,
    half: Vec<Col>,
    half_tmp: Vec<Col>,
    hw: usize,
    hh: usize,
    quarter: Vec<Col>,
    quarter_tmp: Vec<Col>,
    qw: usize,
    qh: usize,
    vignette: Vec<f32>,
    out_pix: Vec<[u8; 3]>,
    prev: Vec<Option<Cell>>,
    text: Vec<Option<(char, Col)>>,
}

impl Canvas {
    pub fn new(cols: usize, rows: usize) -> Canvas {
        let mut c = Canvas {
            w: 0,
            h: 0,
            cols: 0,
            rows: 0,
            buf: vec![],
            cam: V2::default(),
            aberration: 0.0,
            flash: Col::default(),
            exposure: 1.15,
            comp: vec![],
            half: vec![],
            half_tmp: vec![],
            hw: 0,
            hh: 0,
            quarter: vec![],
            quarter_tmp: vec![],
            qw: 0,
            qh: 0,
            vignette: vec![],
            out_pix: vec![],
            prev: vec![],
            text: vec![],
        };
        c.resize(cols, rows);
        c
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(4);
        let rows = rows.max(2);
        self.cols = cols;
        self.rows = rows;
        self.w = cols;
        self.h = rows * 2;
        let n = self.w * self.h;
        self.buf = vec![Col::default(); n];
        self.comp = vec![Col::default(); n];
        self.out_pix = vec![[0; 3]; n];
        self.hw = self.w.div_ceil(2);
        self.hh = self.h.div_ceil(2);
        self.half = vec![Col::default(); self.hw * self.hh];
        self.half_tmp = self.half.clone();
        self.qw = self.hw.div_ceil(2);
        self.qh = self.hh.div_ceil(2);
        self.quarter = vec![Col::default(); self.qw * self.qh];
        self.quarter_tmp = self.quarter.clone();
        self.vignette = Vec::with_capacity(n);
        for y in 0..self.h {
            for x in 0..self.w {
                let dx = (x as f32 + 0.5) / self.w as f32 * 2.0 - 1.0;
                let dy = (y as f32 + 0.5) / self.h as f32 * 2.0 - 1.0;
                let d = (dx * dx * 0.8 + dy * dy * 0.9).min(2.0);
                self.vignette.push(1.0 - d * 0.32);
            }
        }
        self.prev = vec![None; cols * rows];
        self.text = vec![None; cols * rows];
    }

    pub fn invalidate(&mut self) {
        for p in self.prev.iter_mut() {
            *p = None;
        }
    }

    pub fn clear_from(&mut self, bg: &[Col]) {
        if bg.len() == self.buf.len() {
            self.buf.copy_from_slice(bg);
        } else {
            self.buf.fill(Col::default());
        }
        self.text.fill(None);
    }

    // ------------------------------------------------------------------ drawing

    #[inline]
    pub fn add_px(&mut self, x: i32, y: i32, c: Col) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let i = y as usize * self.w + x as usize;
            self.buf[i] += c;
        }
    }

    /// Bilinear sub-pixel splat (anti-aliased point).
    #[inline]
    pub fn splat(&mut self, p: V2, c: Col) {
        let x = p.x + self.cam.x - 0.5;
        let y = p.y + self.cam.y - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let (xi, yi) = (x0 as i32, y0 as i32);
        self.add_px(xi, yi, c * ((1.0 - fx) * (1.0 - fy)));
        self.add_px(xi + 1, yi, c * (fx * (1.0 - fy)));
        self.add_px(xi, yi + 1, c * ((1.0 - fx) * fy));
        self.add_px(xi + 1, yi + 1, c * (fx * fy));
    }

    pub fn line(&mut self, a: V2, b: V2, ca: Col, cb: Col) {
        let d = b - a;
        let len = d.len();
        let n = (len / 0.5).ceil().max(1.0) as usize;
        let k = len / (n as f32 + 1.0);
        let k = k.max(0.25);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.splat(a + d * t, ca.lerp(cb, t) * k);
        }
    }

    pub fn glow(&mut self, p: V2, r: f32, c: Col) {
        let cx = p.x + self.cam.x;
        let cy = p.y + self.cam.y;
        let x0 = (cx - r).floor() as i32;
        let x1 = (cx + r).ceil() as i32;
        let y0 = (cy - r).floor() as i32;
        let y1 = (cy + r).ceil() as i32;
        let inv = 1.0 / r.max(0.01);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d = (dx * dx + dy * dy).sqrt() * inv;
                if d < 1.0 {
                    let f = 1.0 - d;
                    self.add_px(x, y, c * (f * f));
                }
            }
        }
    }

    pub fn ring(&mut self, p: V2, r: f32, c: Col) {
        if r <= 0.1 {
            return;
        }
        let circ = std::f32::consts::TAU * r;
        let n = ((circ / 0.6) as usize).max(10);
        let k = circ / n as f32;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            self.splat(p + V2::from_angle(a) * r, c * k);
        }
    }

    pub fn poly_outline(&mut self, center: V2, r: f32, sides: usize, ang: f32, c: Col) {
        let mut prev = center + V2::from_angle(ang) * r;
        for i in 1..=sides {
            let a = ang + i as f32 / sides as f32 * std::f32::consts::TAU;
            let p = center + V2::from_angle(a) * r;
            self.line(prev, p, c, c);
            prev = p;
        }
    }

    pub fn poly_fill(&mut self, pts: &[V2], c: Col) {
        if pts.len() < 3 {
            return;
        }
        let cam = self.cam;
        let mut miny = f32::MAX;
        let mut maxy = f32::MIN;
        for p in pts {
            miny = miny.min(p.y + cam.y);
            maxy = maxy.max(p.y + cam.y);
        }
        let y0 = (miny.floor() as i32).max(0);
        let y1 = (maxy.ceil() as i32).min(self.h as i32 - 1);
        let mut xs: Vec<f32> = Vec::with_capacity(16);
        for y in y0..=y1 {
            let sy = y as f32 + 0.5;
            xs.clear();
            let n = pts.len();
            for i in 0..n {
                let a = pts[i] + cam;
                let b = pts[(i + 1) % n] + cam;
                if (a.y > sy) != (b.y > sy) {
                    xs.push(a.x + (sy - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let xa = (pair[0] - 0.5).ceil().max(0.0) as i32;
                let xb = ((pair[1] - 0.5).floor() as i32).min(self.w as i32 - 1);
                for x in xa..=xb {
                    self.add_px(x, y, c);
                }
            }
        }
    }

    /// Pixel-font text centred on `center`. `colf(char_index, row) -> colour`.
    pub fn text_fx(&mut self, s: &str, center: V2, scale: i32, colf: impl Fn(usize, usize) -> Col) {
        let scale = scale.max(1);
        let tw = font::text_width(s) as i32 * scale;
        let th = 7 * scale;
        let ox = (center.x + self.cam.x).round() as i32 - tw / 2;
        let oy = (center.y + self.cam.y).round() as i32 - th / 2;
        for (ci, ch) in s.chars().enumerate() {
            let Some(g) = font::glyph(ch) else { continue };
            let gx = ox + ci as i32 * 6 * scale;
            for (ry, row) in g.iter().enumerate() {
                let c = colf(ci, ry);
                for (rx, b) in row.bytes().enumerate() {
                    if b == b'#' {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                self.add_px(gx + rx as i32 * scale + sx, oy + ry as i32 * scale + sy, c);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn text(&mut self, s: &str, center: V2, scale: i32, c: Col) {
        self.text_fx(s, center, scale, |_, _| c);
    }

    /// Terminal-character text overlay (crisp HUD text).
    pub fn put_text(&mut self, col: i32, row: i32, s: &str, fg: Col) {
        if row < 0 || row as usize >= self.rows {
            return;
        }
        for (i, ch) in s.chars().enumerate() {
            let x = col + i as i32;
            if x < 0 || x as usize >= self.cols {
                continue;
            }
            let idx = row as usize * self.cols + x as usize;
            self.text[idx] = if ch == ' ' { None } else { Some((ch, fg)) };
        }
    }

    pub fn put_text_centered(&mut self, row: i32, s: &str, fg: Col) {
        let n = s.chars().count() as i32;
        self.put_text((self.cols as i32 - n) / 2, row, s, fg);
    }

    // ------------------------------------------------------------- post-fx

    fn blur(buf: &mut [Col], tmp: &mut [Col], w: usize, h: usize, k: &[f32]) {
        let r = (k.len() / 2) as i32;
        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let mut acc = Col::default();
                for (i, wgt) in k.iter().enumerate() {
                    let sx = (x as i32 + i as i32 - r).clamp(0, w as i32 - 1) as usize;
                    acc += buf[row + sx] * *wgt;
                }
                tmp[row + x] = acc;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let mut acc = Col::default();
                for (i, wgt) in k.iter().enumerate() {
                    let sy = (y as i32 + i as i32 - r).clamp(0, h as i32 - 1) as usize;
                    acc += tmp[sy * w + x] * *wgt;
                }
                buf[y * w + x] = acc;
            }
        }
    }

    #[inline]
    fn sample(buf: &[Col], w: usize, h: usize, fx: f32, fy: f32) -> Col {
        let fx = fx.clamp(0.0, w as f32 - 1.0);
        let fy = fy.clamp(0.0, h as f32 - 1.0);
        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(w - 1);
        let y1 = (y0 + 1).min(h - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let a = buf[y0 * w + x0].lerp(buf[y0 * w + x1], tx);
        let b = buf[y1 * w + x0].lerp(buf[y1 * w + x1], tx);
        a.lerp(b, ty)
    }

    pub fn finalize(&mut self) {
        let (w, h, hw, hh, qw, qh) = (self.w, self.h, self.hw, self.hh, self.qw, self.qh);
        // bright pass + downsample to half res
        for y in 0..hh {
            for x in 0..hw {
                let mut acc = Col::default();
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(w - 1);
                    let sy = (y * 2 + dy).min(h - 1);
                    let c = self.buf[sy * w + sx];
                    acc += rgb((c.r - 0.22).max(0.0), (c.g - 0.22).max(0.0), (c.b - 0.22).max(0.0));
                }
                self.half[y * hw + x] = acc * 0.25;
            }
        }
        let k5 = [0.06, 0.24, 0.40, 0.24, 0.06];
        Self::blur(&mut self.half, &mut self.half_tmp, hw, hh, &k5);
        for y in 0..qh {
            for x in 0..qw {
                let mut acc = Col::default();
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(hw - 1);
                    let sy = (y * 2 + dy).min(hh - 1);
                    acc += self.half[sy * hw + sx];
                }
                self.quarter[y * qw + x] = acc * 0.25;
            }
        }
        let k7 = [0.03, 0.11, 0.22, 0.28, 0.22, 0.11, 0.03];
        Self::blur(&mut self.quarter, &mut self.quarter_tmp, qw, qh, &k7);
        Self::blur(&mut self.quarter, &mut self.quarter_tmp, qw, qh, &k7);

        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let hb = Self::sample(&self.half, hw, hh, (x as f32 + 0.5) / 2.0 - 0.5, (y as f32 + 0.5) / 2.0 - 0.5);
                let qb = Self::sample(&self.quarter, qw, qh, (x as f32 + 0.5) / 4.0 - 0.5, (y as f32 + 0.5) / 4.0 - 0.5);
                self.comp[i] = self.buf[i] + hb * 1.1 + qb * 1.6 + self.flash;
            }
        }

        let ab = self.aberration.round() as i32;
        let e = self.exposure;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let c = if ab != 0 {
                    let xr = (x as i32 + ab).clamp(0, w as i32 - 1) as usize;
                    let xb = (x as i32 - ab).clamp(0, w as i32 - 1) as usize;
                    rgb(self.comp[y * w + xr].r, self.comp[i].g, self.comp[y * w + xb].b)
                } else {
                    self.comp[i]
                };
                let v = self.vignette[i];
                let tm = |c: f32| ((1.0 - (-c * e).exp()) * v * 255.0).clamp(0.0, 255.0) as u8;
                self.out_pix[i] = [tm(c.r), tm(c.g), tm(c.b)];
            }
        }
    }

    // ------------------------------------------------------------- output

    pub fn present(&mut self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"\x1b[?2026h");
        let mut cursor: Option<(usize, usize)> = None;
        let mut last_fg: Option<[u8; 3]> = None;
        let mut last_bg: Option<[u8; 3]> = None;
        // Skip cells whose colour changed imperceptibly (tolerance grows with brightness).
        let close = |a: [u8; 3], b: [u8; 3]| {
            (0..3).all(|i| {
                let tol = 3 + (a[i].max(b[i]) as i32) / 20;
                (a[i] as i32 - b[i] as i32).abs() <= tol
            })
        };
        for row in 0..self.rows {
            for col in 0..self.cols {
                let idx = row * self.cols + col;
                let top = self.out_pix[(row * 2) * self.w + col];
                let bot = if row * 2 + 1 < self.h { self.out_pix[(row * 2 + 1) * self.w + col] } else { top };
                let cell = match self.text[idx] {
                    Some((ch, fg)) => {
                        let bg = [
                            ((top[0] as u16 + bot[0] as u16) / 5) as u8,
                            ((top[1] as u16 + bot[1] as u16) / 5) as u8,
                            ((top[2] as u16 + bot[2] as u16) / 5) as u8,
                        ];
                        let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
                        Cell { top: bg, bot: bg, fg: [f(fg.r), f(fg.g), f(fg.b)], ch }
                    }
                    None => Cell { top, bot, fg: top, ch: '▀' },
                };
                if let Some(p) = self.prev[idx] {
                    if p.ch == cell.ch && close(p.top, cell.top) && close(p.bot, cell.bot) && close(p.fg, cell.fg) {
                        continue;
                    }
                }
                self.prev[idx] = Some(cell);
                if cursor != Some((row, col)) {
                    let _ = write!(out, "\x1b[{};{}H", row + 1, col + 1);
                }
                let (fg, bg) = if cell.ch == '▀' { (cell.top, cell.bot) } else { (cell.fg, cell.top) };
                // Full-block optimisation: when both halves match use a space with bg only.
                let (ch, fg) = if cell.ch == '▀' && fg == bg { (' ', last_fg.unwrap_or(fg)) } else { (cell.ch, fg) };
                if last_fg != Some(fg) && last_bg != Some(bg) {
                    let _ = write!(out, "\x1b[38;2;{};{};{};48;2;{};{};{}m", fg[0], fg[1], fg[2], bg[0], bg[1], bg[2]);
                } else if last_fg != Some(fg) {
                    let _ = write!(out, "\x1b[38;2;{};{};{}m", fg[0], fg[1], fg[2]);
                } else if last_bg != Some(bg) {
                    let _ = write!(out, "\x1b[48;2;{};{};{}m", bg[0], bg[1], bg[2]);
                }
                last_fg = Some(fg);
                last_bg = Some(bg);
                let mut b = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                cursor = if col + 1 < self.cols { Some((row, col + 1)) } else { None };
            }
        }
        out.extend_from_slice(b"\x1b[?2026l");
    }
}
