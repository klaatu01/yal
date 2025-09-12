use anyhow::{Context, Result, anyhow};
use lightsky::{Lightsky, SpaceId, WindowId};
use std::env;

/* ----------------------------------- Main ----------------------------------- */

fn main() -> Result<()> {
    env_logger::init();

    log::info!("Determining target space ID from command line...");

    let wid: i32 = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("usage: move_window <WINDOW_ID> <FROM_SPACE_ID> <TO_SPACE_ID>"))?
        .parse()
        .context("WINDOW_ID must be an integer (e.g., 1234)")?;

    let wid = WindowId(wid);

    let from_sid: u64 = env::args()
        .nth(2)
        .ok_or_else(|| anyhow!("usage: move_window <WINDOW_ID> <FROM_SPACE_ID> <TO_SPACE_ID>"))?
        .parse()
        .context("SPACE_ID must be an integer (e.g., 5)")?;

    let from_sid = SpaceId(from_sid);

    let to_sid: u64 = env::args()
        .nth(3)
        .ok_or_else(|| anyhow!("usage: move_window <WINDOW_ID> <FROM_SPACE_ID> <TO_SPACE_ID>"))?
        .parse()
        .context("SPACE_ID must be an integer (e.g., 5)")?;

    let to_sid = SpaceId(to_sid);

    let sky = Lightsky::new()?;

    log::info!("Moving window {wid:?} from space {from_sid:?} to space {to_sid:?}...");

    sky.move_window_to_space(wid, from_sid, to_sid)?;

    Ok(())
}
