use std::{env, fs};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sdl2;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;

extern crate chip;

fn main() -> Result<(), String> {
    const BG_COLOR: Color = Color::BLACK;
    const FG_COLOR: Color = Color::GREEN;

    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("Usage: ./{} {{ROM_PATH}} {{REFRESH_HZ}} {{SCALE}}", args[0]);
        return Err("Invalid number of arguments.".parse().unwrap());
    }

    let rom_path = args[1].as_str();
    let hz = args[2].parse::<u64>().expect("Could not parse refresh rate.");
    let scale = args[3].parse::<u32>().expect("Could not parse scale.");

    let sdl = sdl2::init()?;
    let vid = sdl.video()?;

    let mut can = vid.window("chip", 64 * scale, 32 * scale)
        .position_centered()
        .build().expect("Could not build window.")
        .into_canvas()
        .build().expect("Could not build canvas.");

    can.set_scale(scale as f32, scale as f32).expect("Could not set canvas scale.");

    can.set_draw_color(BG_COLOR);
    can.clear();
    can.present();

    let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).expect("System time is pre-epoch.").as_secs();
    let mut core = chip::ChipState::new(unix_time);

    let rom = fs::read(Path::new(rom_path)).expect("Could not open ROM.");
    core.load(rom.as_slice());

    let mut events = sdl.event_pump()?;

    loop {
        let input = input(&mut events);
        if input.is_none() {
            break;
        }

        let (down, up) = input.unwrap();

        core.keys |= down;

        match core.tick() {
            Err(e) => {
                println!("{:?}", e);
                break;
            }
            _ => {
                can.set_draw_color(BG_COLOR);
                can.clear();
                can.set_draw_color(FG_COLOR);

                for row in 0..32 {
                    for column in 0..64 {
                        let fbuf = core.get_fbuf();
                        let pixel = (fbuf[row as usize] >> column) & 1;
                        if pixel == 1 {
                            can.draw_point(Point::new(63 - column, row)).expect("Failed to draw pixel.");
                        }
                    }
                }
            }
        }

        core.keys &= !up;
        can.present();
        sleep(Duration::from_micros(1000000 / hz));
    }

    Ok(())
}

fn input(pump: &mut EventPump) -> Option<(u16, u16)> {
    let mut down = 0;
    let mut up = 0;

    for event in pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return None,
            Event::KeyDown { keycode: Some(result), .. } => {
                match result {
                    Keycode::Num0 => down |= 1,
                    Keycode::Num1 => down |= 1 << 1,
                    Keycode::Num2 => down |= 1 << 2,
                    Keycode::Num3 => down |= 1 << 3,
                    Keycode::Num4 => down |= 1 << 4,
                    Keycode::Num5 => down |= 1 << 5,
                    Keycode::Num6 => down |= 1 << 6,
                    Keycode::Num7 => down |= 1 << 7,
                    Keycode::Num8 => down |= 1 << 8,
                    Keycode::Num9 => down |= 1 << 9,
                    Keycode::A => down |= 1 << 10,
                    Keycode::B => down |= 1 << 11,
                    Keycode::C => down |= 1 << 12,
                    Keycode::D => down |= 1 << 13,
                    Keycode::E => down |= 1 << 14,
                    Keycode::F => down |= 1 << 15,
                    _ => ()
                }
            }
            Event::KeyUp { keycode: Some(result), .. } => {
                match result {
                    Keycode::Num0 => up |= 1 << 0,
                    Keycode::Num1 => up |= 1 << 1,
                    Keycode::Num2 => up |= 1 << 2,
                    Keycode::Num3 => up |= 1 << 3,
                    Keycode::Num4 => up |= 1 << 4,
                    Keycode::Num5 => up |= 1 << 5,
                    Keycode::Num6 => up |= 1 << 6,
                    Keycode::Num7 => up |= 1 << 7,
                    Keycode::Num8 => up |= 1 << 8,
                    Keycode::Num9 => up |= 1 << 9,
                    Keycode::A => up |= 1 << 10,
                    Keycode::B => up |= 1 << 11,
                    Keycode::C => up |= 1 << 12,
                    Keycode::D => up |= 1 << 13,
                    Keycode::E => up |= 1 << 14,
                    Keycode::F => up |= 1 << 15,
                    _ => ()
                }
            }
            _ => ()
        }
    }

    Some((down, up))
}
