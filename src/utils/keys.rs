use leptos::ev::KeyboardEvent;

fn norm_token(t: &str) -> &str {
    match t {
        "control" | "ctrl" => "ctrl",
        "alt" | "option" | "opt" => "alt",
        "shift" => "shift",
        "cmd" | "command" | "meta" | "super" | "win" => "cmd",
        "esc" | "escape" => "esc",
        "enter" | "return" => "enter",
        "space" => "space",
        "pgup" | "pageup" => "pageup",
        "pgdn" | "pagedown" => "pagedown",
        "arrowup" | "up" => "up",
        "arrowdown" | "down" => "down",
        "arrowleft" | "left" => "left",
        "arrowright" | "right" => "right",
        "plus" | "+" => "plus",
        _ => t,
    }
}

pub fn normalize_combo_string(s: &str) -> String {
    let mut parts: Vec<String> = s
        .split('+')
        .map(|p| norm_token(&p.trim().to_ascii_lowercase()).to_string())
        .collect();

    let mut mods: Vec<String> = vec![];
    let mut key: Option<String> = None;

    for p in parts.drain(..) {
        match p.as_str() {
            "ctrl" | "alt" | "shift" | "cmd" => {
                if !mods.contains(&p) {
                    mods.push(p.to_string())
                }
            }
            other => key = Some(other.to_string()),
        }
    }

    mods.sort_unstable_by(|a, b| {
        ["ctrl", "alt", "shift", "cmd"]
            .iter()
            .position(|x| x == a)
            .cmp(&["ctrl", "alt", "shift", "cmd"].iter().position(|x| x == b))
    });

    let k = key.unwrap_or_default();
    if mods.is_empty() {
        k
    } else {
        format!("{}+{}", mods.join("+"), k)
    }
}

pub fn combo_from_event(ev: &KeyboardEvent) -> Option<String> {
    let raw_key = ev.key();
    let lower = raw_key.to_ascii_lowercase();

    let key = match lower.as_str() {
        "shift" | "control" | "alt" | "meta" => return None,
        "escape" => "esc".to_string(),
        "enter" | "return" => "enter".to_string(),
        " " | "spacebar" | "space" => "space".to_string(),
        "arrowup" => "up".to_string(),
        "arrowdown" => "down".to_string(),
        "arrowleft" => "left".to_string(),
        "arrowright" => "right".to_string(),
        "+" => "plus".to_string(),
        k if k.starts_with('f') && k.len() <= 3 && k[1..].chars().all(|c| c.is_ascii_digit()) => {
            k.to_string()
        }
        k if k.len() == 1 => k.to_string(),
        "tab" | "backspace" | "delete" | "insert" | "home" | "end" | "pageup" | "pagedown"
        | "minus" | "equals" | "comma" | "period" | "slash" | "backslash" | "semicolon"
        | "quote" | "bracketleft" | "bracketright" | "grave" => lower.clone(),
        _ => lower.clone(),
    };

    let mut mods: Vec<&str> = vec![];
    if ev.ctrl_key() {
        mods.push("ctrl");
    }
    if ev.alt_key() {
        mods.push("alt");
    }
    if ev.shift_key() {
        mods.push("shift");
    }
    if ev.meta_key() {
        mods.push("cmd");
    }

    mods.sort_unstable_by(|a, b| {
        ["ctrl", "alt", "shift", "cmd"]
            .iter()
            .position(|x| x == a)
            .cmp(&["ctrl", "alt", "shift", "cmd"].iter().position(|x| x == b))
    });

    Some(if mods.is_empty() {
        key
    } else {
        format!("{}+{}", mods.join("+"), key)
    })
}
