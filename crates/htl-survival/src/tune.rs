//! Tuning `survival.toml` while the game runs: one value at a time, by its path, in the text
//! itself (so its comments and layout stay as they are and the result can be kept).
//!
//! A path names tables and keys with dots; an array's element by its number (from 0) or by its
//! `key`: `combat.chain_after`, `bash.stamina`, `moveset.one_hand.light.2.lunge`,
//! `weapon.axe.windup`.

use anyhow::{anyhow, bail, Result};
use toml_edit::{DocumentMut, Item, Table, Value};

/// The text with `path` set to `value` (TOML: `0.3`, `[4, 7]`, `true`, `"text"`; anything that
/// isn't TOML is taken as text), and what it was before (`None`: it wasn't set, so it had its
/// default).
pub fn set(text: &str, path: &str, value: &str) -> Result<(String, Option<String>)> {
    let mut doc: DocumentMut = text.parse()?;
    let segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    if segs.is_empty() {
        bail!("name a value, like combat.chain_after");
    }
    let new: Value = value.trim().parse().unwrap_or_else(|_| Value::from(value.trim()));
    let old = set_in_table(doc.as_table_mut(), &segs, new)?;
    Ok((doc.to_string(), old))
}

/// What `path` holds in the text, if it's set.
pub fn get(text: &str, path: &str) -> Result<Option<String>> {
    let table: toml::Table = toml::from_str(text)?;
    let mut at = toml::Value::Table(table);
    for seg in path.split('.').filter(|s| !s.is_empty()) {
        let next = match &at {
            toml::Value::Table(t) => t.get(seg).cloned(),
            toml::Value::Array(a) => {
                let k = index_of(a.iter().map(|e| e.get("key").and_then(|k| k.as_str())), seg)?;
                a.get(k).cloned()
            }
            _ => bail!("{seg}: not a table or a list"),
        };
        match next {
            Some(v) => at = v,
            None => return Ok(None),
        }
    }
    Ok(Some(match at {
        toml::Value::String(s) => format!("{s:?}"),
        toml::Value::Table(_) => "(a table)".to_string(),
        other => other.to_string(),
    }))
}

/// An element by its number or its `key`.
fn index_of<'a>(keys: impl Iterator<Item = Option<&'a str>>, seg: &str) -> Result<usize> {
    let keys: Vec<Option<&str>> = keys.collect();
    if let Ok(n) = seg.parse::<usize>() {
        if n < keys.len() {
            return Ok(n);
        }
        bail!("there are only {} of them", keys.len());
    }
    keys.iter().position(|k| *k == Some(seg)).ok_or_else(|| anyhow!("no {seg}"))
}

fn set_in_table(t: &mut Table, segs: &[&str], new: Value) -> Result<Option<String>> {
    let seg = segs[0];
    if segs.len() == 1 {
        return Ok(match t.get_mut(seg) {
            Some(Item::Value(v)) => Some(replace(v, new)),
            Some(Item::None) | None => {
                t.insert(seg, Item::Value(new));
                None
            }
            Some(_) => bail!("{seg} is a whole table: name a value in it"),
        });
    }
    if t.get(seg).is_none() {
        let mut fresh = Table::new();
        fresh.set_implicit(false);
        t.insert(seg, Item::Table(fresh));
    }
    match t.get_mut(seg) {
        Some(Item::Table(inner)) => set_in_table(inner, &segs[1..], new),
        Some(Item::ArrayOfTables(a)) => {
            let k = index_of(a.iter().map(|t| t.get("key").and_then(|k| k.as_str())), segs[1])?;
            let inner = a.get_mut(k).ok_or_else(|| anyhow!("no {}", segs[1]))?;
            if segs.len() == 2 {
                bail!("{}.{} is a whole table: name a value in it", seg, segs[1]);
            }
            set_in_table(inner, &segs[2..], new)
        }
        Some(Item::Value(v)) => set_in_value(v, &segs[1..], new),
        _ => bail!("no {seg}"),
    }
}

fn set_in_value(v: &mut Value, segs: &[&str], new: Value) -> Result<Option<String>> {
    let seg = segs[0];
    match v {
        Value::InlineTable(t) => {
            if segs.len() == 1 {
                return Ok(match t.get_mut(seg) {
                    Some(old) => Some(replace(old, new)),
                    None => {
                        t.insert(seg, new);
                        None
                    }
                });
            }
            let inner = t.get_mut(seg).ok_or_else(|| anyhow!("no {seg}"))?;
            set_in_value(inner, &segs[1..], new)
        }
        Value::Array(a) => {
            let k = index_of(a.iter().map(|e| e.as_inline_table().and_then(|t| t.get("key")).and_then(|k| k.as_str())), seg)?;
            let inner = a.get_mut(k).ok_or_else(|| anyhow!("no {seg}"))?;
            if segs.len() == 1 {
                return Ok(Some(replace(inner, new)));
            }
            set_in_value(inner, &segs[1..], new)
        }
        _ => bail!("{seg}: not a table or a list"),
    }
}

/// Put `new` in `slot`, keeping the slot's spacing and comment; the old value as text.
fn replace(slot: &mut Value, mut new: Value) -> String {
    let old = slot.clone().decorated("", "").to_string();
    *new.decor_mut() = slot.decor().clone();
    *slot = new;
    old
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# The rules\n[combat]\n# chains\nchain_after = 0.25 # a share\n\n[[moveset]]\nkey = \"one_hand\"\nlight = [\n  { clip = 17, lunge = 0.7 },\n  { clip = 57, lunge = 1.0 },\n]\n\n[bash]\ndamage = [4, 7]\n";

    #[test]
    fn a_value_changes_in_place_keeping_its_comments() {
        let (text, old) = set(TEXT, "combat.chain_after", "0.3").unwrap();
        assert_eq!(old.as_deref(), Some("0.25"));
        assert!(text.contains("chain_after = 0.3 # a share"), "{text}");
        assert!(text.contains("# chains"));
        assert_eq!(get(&text, "combat.chain_after").unwrap().as_deref(), Some("0.3"));
    }

    #[test]
    fn a_blow_in_a_set_by_key_and_number_and_a_value_not_yet_set() {
        let (text, old) = set(TEXT, "moveset.one_hand.light.1.lunge", "1.4").unwrap();
        assert_eq!(old.as_deref(), Some("1.0"));
        assert!(text.contains("{ clip = 57, lunge = 1.4 }"), "{text}");
        let (text, old) = set(&text, "moveset.0.light.0.windup", "0.9").unwrap();
        assert_eq!(old, None);
        assert_eq!(get(&text, "moveset.one_hand.light.0.windup").unwrap().as_deref(), Some("0.9"));
        let (text, _) = set(&text, "bash.damage", "[5, 8]").unwrap();
        assert_eq!(get(&text, "bash.damage").unwrap().as_deref(), Some("[5, 8]"));
        let (text, old) = set(&text, "kick.stamina", "10").unwrap();
        assert_eq!(old, None);
        assert_eq!(get(&text, "kick.stamina").unwrap().as_deref(), Some("10"));
    }

    #[test]
    fn a_bad_path_says_so() {
        assert!(set(TEXT, "moveset.two_hand.light.0.lunge", "1").is_err());
        assert!(set(TEXT, "moveset.one_hand.light.5.lunge", "1").is_err());
        assert!(set(TEXT, "combat", "1").is_err());
        assert_eq!(get(TEXT, "combat.nothing").unwrap(), None);
    }

    #[test]
    fn the_real_rules_tune_and_take_effect() {
        let c = crate::Content::builtin();
        let (c2, old) = c.tuned("combat.chain_after", "0.4").unwrap();
        assert_eq!(old.as_deref(), Some("0.25"));
        assert!((c2.combat.chain_after - 0.4).abs() < 1e-6);
        let (c3, _) = c2.tuned("moveset.one_hand.light.2.lunge", "1.6").unwrap();
        assert!((c3.movesets[0].light[2].lunge - 1.6).abs() < 1e-6);
        assert!(c.tuned("combat.chain_after", "\"lots\"").is_err(), "a word where a number goes");
        let (c4, _) = c.tuned("mob.wolf.pattern.1.weight", "0.5").unwrap();
        let b = c4.mob("wolf").unwrap();
        assert!((c4.mobs[b].patterns[1].weight - 0.5).abs() < 1e-6);
        let (c6, _) = c.tuned("mob.grunt.pattern.0.steps.1.windup", "0.5").unwrap();
        let g = c6.mob("grunt").unwrap();
        assert!((c6.mobs[g].patterns[0].steps[1].windup.unwrap() - 0.5).abs() < 1e-6);
        let (c5, _) = c.tuned("dodge.secs", "0.5").unwrap();
        assert!((c5.dodge.secs - 0.5).abs() < 1e-6);
    }
}
