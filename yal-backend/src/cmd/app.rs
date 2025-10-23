use std::path::Path;
use walkdir::WalkDir;
use yal_core::AppInfo;

fn read_app_name(bundle_path: &Path) -> String {
    let plist_path = bundle_path.join("Contents").join("Info.plist");
    if let Ok(v) = plist::Value::from_file(&plist_path) {
        if let Some(d) = v.as_dictionary() {
            for key in ["CFBundleDisplayName", "CFBundleName", "Bundle name"] {
                if let Some(pl) = d.get(key).and_then(|v| v.as_string()) {
                    return pl.to_string();
                }
            }
        }
    }

    bundle_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

fn collect_apps_in(dir: &Path) -> Vec<AppInfo> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(dir).max_depth(2).into_iter().flatten() {
        let path = entry.path();
        if path.is_dir() && path.extension().and_then(|e| e.to_str()) == Some("app") {
            let name = read_app_name(path);
            out.push(AppInfo {
                name,
                path: path.to_string_lossy().into_owned(),
            });
        }
    }
    out
}

fn collect_system_preferences() -> Vec<AppInfo> {
    const PREFS: &[(&str, &str)] = &[
        ("Bluetooth", "x-apple.systempreferences:com.apple.Bluetooth"),
        (
            "Network",
            "x-apple.systempreferences:com.apple.preference.network",
        ),
        (
            "Privacy & Security",
            "x-apple.systempreferences:com.apple.preference.security",
        ),
        (
            "Notifications",
            "x-apple.systempreferences:com.apple.preference.notifications",
        ),
        (
            "Sound",
            "x-apple.systempreferences:com.apple.preference.sound",
        ),
        (
            "Displays",
            "x-apple.systempreferences:com.apple.preference.displays",
        ),
        (
            "Keyboard",
            "x-apple.systempreferences:com.apple.preference.keyboard",
        ),
        (
            "Mouse",
            "x-apple.systempreferences:com.apple.preference.mouse",
        ),
        (
            "Trackpad",
            "x-apple.systempreferences:com.apple.preference.trackpad",
        ),
        (
            "Battery / Energy Saver",
            "x-apple.systempreferences:com.apple.preference.energysaver",
        ),
        (
            "Date & Time",
            "x-apple.systempreferences:com.apple.preference.datetime",
        ),
        (
            "Accessibility",
            "x-apple.systempreferences:com.apple.preference.universalaccess",
        ),
        ("Wi-Fi", "x-apple.systempreferences:com.apple.WiFiSettings"),
    ];

    PREFS
        .iter()
        .map(|(label, uri)| AppInfo {
            name: format!("system preferences - {}", label),
            path: (*uri).to_string(),
        })
        .collect()
}

pub fn get_app_info() -> Result<Vec<AppInfo>, String> {
    let mut apps: Vec<AppInfo> = Vec::new();
    apps.append(&mut collect_apps_in(Path::new("/Applications")));
    apps.append(&mut collect_apps_in(Path::new("/System/Applications")));
    apps.append(&mut collect_system_preferences());
    if let Some(home) = dirs::home_dir() {
        apps.append(&mut collect_apps_in(&home.join("Applications")));
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}
