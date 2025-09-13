use anyhow::{Context, Result, anyhow};
use lightsky::{Lightsky, SpaceId, WindowId};
use std::env;

/* ----------------------------------- Main ----------------------------------- */

fn main() -> Result<()> {
    env_logger::init();

    log::info!("Determining target space ID from command line...");

    let display_id: String = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("usage: change_space <DISPLAY_ID> <SPACE_ID> <TO_SPACE_ID>"))?
        .parse()
        .context("DISPLAY_ID must be an integer (e.g., 1234)")?;

    let sid: u64 = env::args()
        .nth(2)
        .ok_or_else(|| anyhow!("usage: change_space <DISPLAY_ID> <SPACE_ID> <TO_SPACE_ID>"))?
        .parse()
        .context("SPACE_ID must be an integer (e.g., 5)")?;

    let sid = SpaceId(sid);

    let to_sid: u64 = env::args()
        .nth(3)
        .ok_or_else(|| anyhow!("usage: change_space <DISPLAY_ID> <SPACE_ID> <TO_SPACE_ID>"))?
        .parse()
        .context("SPACE_ID must be an integer (e.g., 5)")?;

    let to_sid = SpaceId(to_sid);

    let sky = Lightsky::new()?;

    sky.change_space(display_id, sid, to_sid)?;

    Ok(())
}
