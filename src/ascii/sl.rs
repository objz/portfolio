use super::steam::*;
use crate::terminal::buffer::{self, LineType};
use crate::terminal::renderer::TerminalRenderer;
use anyhow::Result;

const SMOKEPTNS: usize = 16;
const SMOKE: [[&str; SMOKEPTNS]; 2] = [
    [
        "(   )", "(    )", "(    )", "(   )", "(  )", "(  )", "( )", "( )", "()", "()", "O", "O",
        "O", "O", "O", " ",
    ],
    [
        "(@@@)", "(@@@@)", "(@@@@)", "(@@@)", "(@@)", "(@@)", "(@)", "(@)", "@@", "@@", "@", "@",
        "@", "@", "@", " ",
    ],
];
const DY: [i32; SMOKEPTNS] = [2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const DX: [i32; SMOKEPTNS] = [-2, -1, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3];

const D51_FRAMES: [[&str; D51HEIGHT + 1]; D51PATTERNS] = [
    [
        D51STR1, D51STR2, D51STR3, D51STR4, D51STR5, D51STR6, D51STR7, D51WHL11, D51WHL12,
        D51WHL13, D51DEL,
    ],
    [
        D51STR1, D51STR2, D51STR3, D51STR4, D51STR5, D51STR6, D51STR7, D51WHL21, D51WHL22,
        D51WHL23, D51DEL,
    ],
    [
        D51STR1, D51STR2, D51STR3, D51STR4, D51STR5, D51STR6, D51STR7, D51WHL31, D51WHL32,
        D51WHL33, D51DEL,
    ],
    [
        D51STR1, D51STR2, D51STR3, D51STR4, D51STR5, D51STR6, D51STR7, D51WHL41, D51WHL42,
        D51WHL43, D51DEL,
    ],
    [
        D51STR1, D51STR2, D51STR3, D51STR4, D51STR5, D51STR6, D51STR7, D51WHL51, D51WHL52,
        D51WHL53, D51DEL,
    ],
    [
        D51STR1, D51STR2, D51STR3, D51STR4, D51STR5, D51STR6, D51STR7, D51WHL61, D51WHL62,
        D51WHL63, D51DEL,
    ],
];

const D51_COAL: [&str; D51HEIGHT + 1] = [
    COAL01, COAL02, COAL03, COAL04, COAL05, COAL06, COAL07, COAL08, COAL09, COAL10, COALDEL,
];

const LOGO_FRAMES: [[&str; LOGOHEIGHT + 1]; LOGOPATTERNS] = [
    [LOGO1, LOGO2, LOGO3, LOGO4, LWHL11, LWHL12, DELLN],
    [LOGO1, LOGO2, LOGO3, LOGO4, LWHL21, LWHL22, DELLN],
    [LOGO1, LOGO2, LOGO3, LOGO4, LWHL31, LWHL32, DELLN],
    [LOGO1, LOGO2, LOGO3, LOGO4, LWHL41, LWHL42, DELLN],
    [LOGO1, LOGO2, LOGO3, LOGO4, LWHL51, LWHL52, DELLN],
    [LOGO1, LOGO2, LOGO3, LOGO4, LWHL61, LWHL62, DELLN],
];

const LOGO_COAL: [&str; LOGOHEIGHT + 1] = [LCOAL1, LCOAL2, LCOAL3, LCOAL4, LCOAL5, LCOAL6, DELLN];
const LOGO_CAR: [&str; LOGOHEIGHT + 1] = [LCAR1, LCAR2, LCAR3, LCAR4, LCAR5, LCAR6, DELLN];

const C51_FRAMES: [[&str; C51HEIGHT + 1]; C51PATTERNS] = [
    [
        C51STR1, C51STR2, C51STR3, C51STR4, C51STR5, C51STR6, C51STR7, C51WH11, C51WH12, C51WH13,
        C51WH14, C51DEL,
    ],
    [
        C51STR1, C51STR2, C51STR3, C51STR4, C51STR5, C51STR6, C51STR7, C51WH21, C51WH22, C51WH23,
        C51WH24, C51DEL,
    ],
    [
        C51STR1, C51STR2, C51STR3, C51STR4, C51STR5, C51STR6, C51STR7, C51WH31, C51WH32, C51WH33,
        C51WH34, C51DEL,
    ],
    [
        C51STR1, C51STR2, C51STR3, C51STR4, C51STR5, C51STR6, C51STR7, C51WH41, C51WH42, C51WH43,
        C51WH44, C51DEL,
    ],
    [
        C51STR1, C51STR2, C51STR3, C51STR4, C51STR5, C51STR6, C51STR7, C51WH51, C51WH52, C51WH53,
        C51WH54, C51DEL,
    ],
    [
        C51STR1, C51STR2, C51STR3, C51STR4, C51STR5, C51STR6, C51STR7, C51WH61, C51WH62, C51WH63,
        C51WH64, C51DEL,
    ],
];

const C51_COAL: [&str; C51HEIGHT + 1] = [
    COALDEL, COAL01, COAL02, COAL03, COAL04, COAL05, COAL06, COAL07, COAL08, COAL09, COAL10,
    COALDEL,
];

#[derive(Clone, Copy)]
struct Options {
    accident: bool,
    fly: bool,
    logo: bool,
    c51: bool,
}

impl Options {
    fn parse(args: &[&str]) -> Self {
        let mut options = Self {
            accident: false,
            fly: false,
            logo: false,
            c51: false,
        };

        for arg in args {
            match *arg {
                "-a" => options.accident = true,
                "-f" => options.fly = true,
                "-l" => options.logo = true,
                "-c" => options.c51 = true,
                _ => {}
            }
        }

        options
    }
}

#[derive(Clone, Copy)]
struct SmokePuff {
    x: i32,
    y: i32,
    ptrn: usize,
    kind: usize,
}

struct Frame {
    width: usize,
    height: usize,
    rows: Vec<Vec<char>>,
}

impl Frame {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            rows: vec![vec![' '; width]; height],
        }
    }

    fn draw(&mut self, x: i32, y: i32, sprite: &str) {
        if y < 0 || y >= self.height as i32 {
            return;
        }

        for (offset, ch) in sprite.chars().enumerate() {
            let col = x + offset as i32;
            if col < 0 || col >= self.width as i32 {
                continue;
            }

            self.rows[y as usize][col as usize] = ch;
        }
    }

    fn flush_to_buffer(self) {
        for row in self.rows {
            let line: String = row.into_iter().collect();
            buffer::add_line(line.trim_end().to_string(), LineType::Output, None);
        }
    }
}

pub async fn animate(renderer: &TerminalRenderer, args: &[&str]) -> Result<()> {
    let options = Options::parse(args);
    let width = renderer.max_chars_per_line().max(1);
    let height = renderer.max_visible_lines().max(1);

    let train_length = if options.logo {
        LOGOLENGTH as i32
    } else if options.c51 {
        C51LENGTH as i32
    } else {
        D51LENGTH as i32
    };

    let mut x = width as i32 - 1;
    let mut tick = 0usize;
    let mut smoke = Vec::<SmokePuff>::new();

    buffer::clear_buffer();
    buffer::reset_scroll();

    while x >= -train_length {
        let mut frame = Frame::new(width, height);

        if options.logo {
            draw_logo_frame(&mut frame, x, options, tick, &mut smoke);
        } else if options.c51 {
            draw_c51_frame(&mut frame, x, options, tick, &mut smoke);
        } else {
            draw_d51_frame(&mut frame, x, options, tick, &mut smoke);
        }

        buffer::clear_buffer();
        frame.flush_to_buffer();
        buffer::reset_scroll();
        renderer.render();

        renderer.sleep(40).await;
        x -= 1;
        tick += 1;
    }

    buffer::clear_buffer();
    buffer::reset_scroll();
    renderer.render();

    Ok(())
}

fn draw_logo_frame(
    frame: &mut Frame,
    x: i32,
    options: Options,
    tick: usize,
    smoke: &mut Vec<SmokePuff>,
) {
    let cols = frame.width as i32;
    let rows = frame.height as i32;
    let frame_idx = (((LOGOLENGTH as i32 + x) / 3).rem_euclid(LOGOPATTERNS as i32)) as usize;

    let (y, py1, py2, py3) = if options.fly {
        let y = (x / 6) + rows - (cols / 6) - LOGOHEIGHT as i32;
        (y, 2, 4, 6)
    } else {
        (rows / 2 - 3, 0, 0, 0)
    };

    for i in 0..=LOGOHEIGHT as i32 {
        let row = i as usize;
        frame.draw(x, y + i, LOGO_FRAMES[frame_idx][row]);
        frame.draw(x + 21, y + i + py1, LOGO_COAL[row]);
        frame.draw(x + 42, y + i + py2, LOGO_CAR[row]);
        frame.draw(x + 63, y + i + py3, LOGO_CAR[row]);
    }

    if options.accident {
        draw_man(frame, x + 14, y + 1, tick);
        draw_man(frame, x + 45, y + 1 + py2, tick);
        draw_man(frame, x + 53, y + 1 + py2, tick);
        draw_man(frame, x + 66, y + 1 + py3, tick);
        draw_man(frame, x + 74, y + 1 + py3, tick);
    }

    draw_smoke(frame, smoke, x + LOGOFUNNEL as i32, y - 1, tick);
}

fn draw_d51_frame(
    frame: &mut Frame,
    x: i32,
    options: Options,
    tick: usize,
    smoke: &mut Vec<SmokePuff>,
) {
    let cols = frame.width as i32;
    let rows = frame.height as i32;
    let frame_idx = (D51LENGTH as i32 + x).rem_euclid(D51PATTERNS as i32) as usize;

    let (y, dy) = if options.fly {
        let y = (x / 7) + rows - (cols / 7) - D51HEIGHT as i32;
        (y, 1)
    } else {
        (rows / 2 - 5, 0)
    };

    for i in 0..=D51HEIGHT as i32 {
        let row = i as usize;
        frame.draw(x, y + i, D51_FRAMES[frame_idx][row]);
        frame.draw(x + 53, y + i + dy, D51_COAL[row]);
    }

    if options.accident {
        draw_man(frame, x + 43, y + 2, tick);
        draw_man(frame, x + 47, y + 2, tick);
    }

    draw_smoke(frame, smoke, x + D51FUNNEL as i32, y - 1, tick);
}

fn draw_c51_frame(
    frame: &mut Frame,
    x: i32,
    options: Options,
    tick: usize,
    smoke: &mut Vec<SmokePuff>,
) {
    let cols = frame.width as i32;
    let rows = frame.height as i32;
    let frame_idx = (C51LENGTH as i32 + x).rem_euclid(C51PATTERNS as i32) as usize;

    let (y, dy) = if options.fly {
        let y = (x / 7) + rows - (cols / 7) - C51HEIGHT as i32;
        (y, 1)
    } else {
        (rows / 2 - 5, 0)
    };

    for i in 0..=C51HEIGHT as i32 {
        let row = i as usize;
        frame.draw(x, y + i, C51_FRAMES[frame_idx][row]);
        frame.draw(x + 55, y + i + dy, C51_COAL[row]);
    }

    if options.accident {
        draw_man(frame, x + 45, y + 3, tick);
        draw_man(frame, x + 49, y + 3, tick);
    }

    draw_smoke(frame, smoke, x + C51FUNNEL as i32, y - 1, tick);
}

fn draw_man(frame: &mut Frame, x: i32, y: i32, tick: usize) {
    const MAN: [[&str; 2]; 2] = [["", "(O)"], ["Help!", "\\O/"]];
    let frame_idx = tick % 2;
    frame.draw(x, y, MAN[frame_idx][0]);
    frame.draw(x, y + 1, MAN[frame_idx][1]);
}

fn draw_smoke(frame: &mut Frame, smoke: &mut Vec<SmokePuff>, x: i32, y: i32, tick: usize) {
    if tick % 4 == 0 {
        smoke.push(SmokePuff {
            x,
            y,
            ptrn: 0,
            kind: tick % 2,
        });
    }

    for puff in smoke.iter_mut() {
        frame.draw(puff.x, puff.y, SMOKE[puff.kind][puff.ptrn]);
    }

    for puff in smoke.iter_mut() {
        let ptrn = puff.ptrn;
        puff.y -= DY[ptrn];
        puff.x += DX[ptrn];
        if puff.ptrn < SMOKEPTNS - 1 {
            puff.ptrn += 1;
        }
    }

    smoke.retain(|puff| puff.ptrn < SMOKEPTNS - 1);
}
