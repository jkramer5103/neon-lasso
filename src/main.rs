mod font;
mod game;
mod math;
mod render;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{self, DisableLineWrap, EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::time::{Duration, Instant};

fn restore(enhanced: bool) {
    let mut out = io::stdout();
    if enhanced {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(out, crossterm::style::ResetColor, EnableLineWrap, cursor::Show, LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

fn main() -> io::Result<()> {
    let mut out = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, cursor::Hide, DisableLineWrap, terminal::Clear(terminal::ClearType::All))?;
    let enhanced = matches!(terminal::supports_keyboard_enhancement(), Ok(true));
    if enhanced {
        let _ = execute!(
            out,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        );
    }
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore(enhanced);
        hook(info);
    }));

    let res = run(enhanced);
    restore(enhanced);
    res
}

fn dir_of(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Char('k') => Some(0),
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('j') => Some(1),
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('h') => Some(2),
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('l') => Some(3),
        _ => None,
    }
}

fn run(enhanced: bool) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let mut canvas = render::Canvas::new(cols as usize, rows as usize);
    let mut game = game::Game::new(canvas.w as f32, canvas.h as f32, enhanced);
    let mut out = io::stdout();
    let mut frame_buf: Vec<u8> = Vec::with_capacity(1 << 20);
    let target = Duration::from_secs_f64(1.0 / 60.0);
    let mut last = Instant::now();
    let stats = std::env::var_os("NEON_LASSO_STATS").is_some();
    let (mut bytes, mut frames, mut work) = (0usize, 0usize, Duration::ZERO);

    loop {
        let frame_start = Instant::now();
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(k) => {
                    let press = matches!(k.kind, KeyEventKind::Press);
                    let repeat = matches!(k.kind, KeyEventKind::Repeat);
                    if k.kind == KeyEventKind::Release {
                        if let Some(d) = dir_of(k.code) {
                            game.release(d);
                        }
                        continue;
                    }
                    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
                        game.quit = true;
                    }
                    if let Some(d) = dir_of(k.code) {
                        game.press(d, repeat);
                        continue;
                    }
                    if !press {
                        continue;
                    }
                    match k.code {
                        KeyCode::Char(' ') => {
                            if game.mode == game::Mode::Title {
                                game.confirm();
                            } else {
                                game.dash();
                            }
                        }
                        KeyCode::Enter => game.confirm(),
                        KeyCode::Char('p') | KeyCode::Char('P') => game.toggle_pause(),
                        KeyCode::Esc => game.back(),
                        KeyCode::Char('q') | KeyCode::Char('Q') => game.quit = true,
                        _ => {}
                    }
                }
                Event::Resize(c, r) => {
                    canvas.resize(c as usize, r as usize);
                    game.resize(canvas.w as f32, canvas.h as f32);
                    execute!(out, terminal::Clear(terminal::ClearType::All))?;
                    canvas.invalidate();
                }
                Event::FocusLost => {
                    if game.mode == game::Mode::Playing {
                        game.toggle_pause();
                    }
                }
                _ => {}
            }
        }
        if game.quit {
            break;
        }
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(1.0 / 20.0);
        last = now;

        game.update(dt);
        game.render(&mut canvas);
        canvas.finalize();
        frame_buf.clear();
        canvas.present(&mut frame_buf);
        out.write_all(&frame_buf)?;
        out.flush()?;
        bytes += frame_buf.len();
        frames += 1;
        work += frame_start.elapsed();

        let elapsed = frame_start.elapsed();
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }
    if stats {
        restore(enhanced);
        eprintln!(
            "{}x{}: {} frames, avg {:.1} KB/frame, avg frame work {:.2} ms",
            canvas.cols, canvas.rows, frames, bytes as f64 / frames.max(1) as f64 / 1024.0, work.as_secs_f64() * 1000.0 / frames.max(1) as f64
        );
        std::thread::sleep(Duration::from_secs(3));
    }
    Ok(())
}
