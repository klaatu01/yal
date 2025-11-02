use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShortcutCommand {
    pub plugin: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub combination: String,
    pub command: ShortcutCommand,
}

impl Serialize for ShortcutCommand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("plugin", &self.plugin)?;
        map.serialize_entry("command", &self.command)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for ShortcutCommand {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SCVisitor;

        impl<'de> Visitor<'de> for SCVisitor {
            type Value = ShortcutCommand;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#"a map with keys "plugin" and "command""#)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut plugin = None;
                let mut command = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "plugin" => plugin = Some(map.next_value()?),
                        "command" => command = Some(map.next_value()?),
                        _ => { let _: de::IgnoredAny = map.next_value()?; }
                    }
                }

                Ok(ShortcutCommand {
                    plugin: plugin.ok_or_else(|| de::Error::missing_field("plugin"))?,
                    command: command.ok_or_else(|| de::Error::missing_field("command"))?,
                })
            }
        }

        deserializer.deserialize_map(SCVisitor)
    }
}

impl Serialize for Shortcut {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.combination, &self.command)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Shortcut {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ShortcutVisitor;

        impl<'de> Visitor<'de> for ShortcutVisitor {
            type Value = Shortcut;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#"a single-entry map like { "<combo>": { plugin, command } }"#)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (combination, command): (String, ShortcutCommand) =
                    map.next_entry()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;

                if map.next_entry::<de::IgnoredAny, de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom(
                        "expected a single-entry map for Shortcut, but found multiple entries",
                    ));
                }

                Ok(Shortcut { combination, command })
            }
        }

        deserializer.deserialize_map(ShortcutVisitor)
    }
}
