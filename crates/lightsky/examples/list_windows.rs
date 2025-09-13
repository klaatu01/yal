#![cfg(target_os = "macos")]

use lightsky::{Lightsky, WindowKindFilter, WindowListOptions};

fn main() -> anyhow::Result<()> {
    let sky = Lightsky::new()?;
    let displays = sky.list_all_spaces()?;
    for display in displays {
        println!("{}:", display);
        for space in display.spaces {
            println!("  Space ID: {}, index: {}", space.id, space.index);
            println!("  Windows:");
            let windows = sky.get_windows_in_space_with_titles(
                space.id,
                WindowListOptions::all(),
                WindowKindFilter::APP,
            )?;
            for window in windows {
                println!("    Window ID: {}", window.info.window_id);
                println!("      Level: {}", window.info.level);
                println!(
                    "      App: {}",
                    window.owner_name.unwrap_or("<No App>".into())
                );
                println!(
                    "      Title: {}",
                    window.title.unwrap_or("<No Title>".into())
                );
                println!("      PID: {}", window.pid);
            }
        }
    }
    Ok(())
}
