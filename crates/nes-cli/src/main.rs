use std::env;
use std::fs;

use nes_core::Nes;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: nes-cli <rom.nes> [--headless] [--frames N]");
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let headless = args.contains(&"--headless".to_string());
    let frames: u64 = args
        .iter()
        .position(|a| a == "--frames")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let rom_data = match fs::read(rom_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading ROM: {e}");
            std::process::exit(1);
        }
    };

    let mut nes = Nes::new();
    match nes.load_rom(&rom_data) {
        Ok(()) => println!("ROM loaded successfully"),
        Err(e) => {
            eprintln!("Error loading ROM: {e:?}");
            std::process::exit(1);
        }
    }

    if headless {
        println!("Running {frames} frames in headless mode...");
        for i in 0..frames {
            nes.run_frame();
            if (i + 1).is_multiple_of(60) {
                println!("Frame {}: CPU cycles = {}", i + 1, nes.cpu_cycles());
            }
        }
        println!("Done. Total CPU cycles: {}", nes.cpu_cycles());
    } else {
        println!("Display mode not implemented. Use --headless for testing.");
    }
}
