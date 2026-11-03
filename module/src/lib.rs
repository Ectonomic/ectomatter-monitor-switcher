//! The Monitor Switcher plugin: a column for each monitor on each game's page, and
//! the changes a game's choices make as it starts. Ectomatter reads the
//! monitors for it (the system interface's `monitors`), makes each change
//! itself, writes down first what each was, and puts it back as the game
//! quits; this plugin only describes the choices and turns them into changes.
//!
//! A monitor is known by the id Ectomatter makes from its EDID, so a choice
//! stays with its monitor whatever port it's on. A choice is kept as
//! "<monitor id>/<what>": brightness, resolution, refresh, hdr or vrr; and
//! "main", the main monitor's id.

use serde_json::{Value, json};

wit_bindgen::generate!({ path: "../../../wit", world: "core" });

use ectomatter::core::system;

/// The monitors as Ectomatter reads them now: the main one first, then by port.
fn monitors() -> Result<Vec<Value>, String> {
    let text = system::monitors()?;
    let mut list: Vec<Value> = serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())?.as_array().cloned().unwrap_or_default();
    list.sort_by_key(|m| (m["main"] != true, m["port"].as_str().unwrap_or("").to_string()));
    Ok(list)
}

/// A monitor's name as a person knows it: its maker's, then its own (the
/// maker left off where its name starts with it).
fn label(m: &Value) -> String {
    let name = m["name"].as_str().unwrap_or("").trim();
    let maker = m["maker"].as_str().unwrap_or("").trim();
    match (maker, name) {
        (_, "") => if maker.is_empty() { "A monitor".into() } else { format!("A {maker} monitor") },
        ("", n) => n.into(),
        (mk, n) if n.to_lowercase().starts_with(&mk.to_lowercase()) => n.into(),
        (mk, n) => format!("{mk} {n}"),
    }
}

fn mode_of(v: &Value) -> Option<(u64, u64, u64)> {
    Some((v["width"].as_u64()?, v["height"].as_u64()?, v["refresh"].as_u64()?))
}

/// A refresh rate in whole hertz, as a monitor's menu says it (239.999 is 240).
fn hz(millihertz: u64) -> u64 {
    (millihertz + 500) / 1000
}

fn res_key(w: u64, h: u64) -> String {
    format!("{w}x{h}")
}

/// Each resolution a monitor has, largest first.
fn resolutions(m: &Value) -> Vec<(u64, u64)> {
    let mut out: Vec<(u64, u64)> = Vec::new();
    for (w, h, _) in m["modes"].as_array().into_iter().flatten().filter_map(mode_of) {
        if !out.contains(&(w, h)) {
            out.push((w, h));
        }
    }
    out.sort_by(|a, b| (b.0 * b.1, b.0).cmp(&(a.0 * a.1, a.0)));
    out
}

/// The rates a monitor has at a resolution, in whole hertz, fastest first.
fn rates_at(m: &Value, w: u64, h: u64) -> Vec<u64> {
    let mut out: Vec<u64> = m["modes"].as_array().into_iter().flatten().filter_map(mode_of).filter(|(mw, mh, _)| (*mw, *mh) == (w, h)).map(|(_, _, r)| hz(r)).collect();
    out.sort_unstable_by(|a, b| b.cmp(a));
    out.dedup();
    out
}

fn current(m: &Value) -> Option<(u64, u64, u64)> {
    mode_of(&m["mode"])
}

fn opt(value: &str, label: &str) -> Value {
    json!({ "value": value, "label": label })
}

fn leave() -> Value {
    opt("leave", "Leave as is")
}

/// A choice drawn as a dropdown, starting at Leave as is.
fn menu(options: Vec<Value>) -> Value {
    json!({ "type": "choice", "menu": true, "options": options, "default": "leave" })
}

/// Under a monitor's name: its mode now, HDR if it's on, and whether it's
/// the main monitor; or that it's off.
fn column_note(m: &Value) -> String {
    if m["enabled"] == false {
        return "Off".into();
    }
    let mut parts = Vec::new();
    if let Some((w, h, r)) = current(m) {
        parts.push(format!("{w} × {h}, {} Hz", hz(r)));
    }
    if m["hdr"] == true {
        parts.push("HDR".into());
    }
    let mut s = parts.join(", ");
    if m["main"] == true {
        s = if s.is_empty() { "Main".into() } else { format!("{s} · Main") };
    }
    s
}

const BRIGHTNESS: [(&str, &str); 6] = [("100", "100%"), ("75", "75%"), ("50", "50%"), ("25", "25%"), ("lowest", "Lowest"), ("off", "Turn off")];

/// The section a page gets: a column for each monitor, a row for each thing
/// it can change (a cell saying why where a monitor can't), then the main
/// monitor, and how it goes.
fn describe(p: &Value) -> Value {
    let page = p["for"].as_str().unwrap_or("game");
    let name = p["name"].as_str().unwrap_or("the game");
    let list = match monitors() {
        Ok(l) => l,
        Err(e) => return json!({ "section": { "label": "Monitor Switcher", "layout": [{ "note": e }] } }),
    };
    if list.is_empty() {
        return json!({ "section": { "label": "Monitor Switcher", "layout": [{ "note": "Ectomatter can’t find a monitor on this computer." }] } });
    }
    let several = list.len() > 1;
    let mut settings = serde_json::Map::new();
    let mut columns = Vec::new();
    let mut rows: Vec<(String, Vec<Value>)> = ["Brightness", "Resolution", "Refresh rate", "HDR", "Variable refresh"].iter().map(|r| (r.to_string(), Vec::new())).collect();
    for m in &list {
        let id = m["id"].as_str().unwrap_or("").to_string();
        columns.push(json!({ "id": id, "label": label(m), "note": column_note(m) }));
        let key = |what: &str| format!("{id}/{what}");
        // brightness: where it can be set, each step; turning it off wherever
        // there are others to carry its windows
        let mut b = vec![leave()];
        if m.get("brightness").is_some() {
            b.extend(BRIGHTNESS.iter().filter(|(v, _)| *v != "off").map(|(v, l)| opt(v, l)));
        }
        if several {
            b.push(opt("off", "Turn off"));
        }
        if b.len() > 1 {
            settings.insert(key("brightness"), menu(b));
            rows[0].1.push(json!(key("brightness")));
        } else {
            rows[0].1.push(json!({ "none": "Its brightness can’t be set" }));
        }
        // its resolutions, and its rates at the resolution it's at
        let res = resolutions(m);
        if res.len() > 1 {
            settings.insert(key("resolution"), menu(std::iter::once(leave()).chain(res.iter().map(|(w, h)| opt(&res_key(*w, *h), &format!("{w} × {h}")))).collect()));
            rows[1].1.push(json!(key("resolution")));
        } else {
            rows[1].1.push(json!({ "none": res.first().map_or("Only one".to_string(), |(w, h)| format!("Only {w} × {h}")) }));
        }
        let (cw, ch) = current(m).map(|(w, h, _)| (w, h)).or_else(|| res.first().copied()).unwrap_or((0, 0));
        let rates = rates_at(m, cw, ch);
        if rates.len() > 1 {
            settings.insert(key("refresh"), menu(std::iter::once(leave()).chain(rates.iter().map(|r| opt(&r.to_string(), &format!("{r} Hz")))).collect()));
            rows[2].1.push(json!(key("refresh")));
        } else {
            rows[2].1.push(json!({ "none": rates.first().map_or("One rate only".to_string(), |r| format!("Only {r} Hz")) }));
        }
        if m.get("hdr").is_some() {
            settings.insert(key("hdr"), menu(vec![leave(), opt("on", "On"), opt("off", "Off")]));
            rows[3].1.push(json!(key("hdr")));
        } else {
            rows[3].1.push(json!({ "none": "No HDR" }));
        }
        if m.get("vrr").is_some() {
            settings.insert(key("vrr"), menu(vec![leave(), opt("always", "Always"), opt("automatic", "Automatic"), opt("off", "Off")]));
            rows[4].1.push(json!(key("vrr")));
        } else {
            rows[4].1.push(json!({ "none": "No variable refresh" }));
        }
    }
    // (a row no monitor can change is left out)
    let rows: Vec<Value> = rows.into_iter().filter(|(_, cells)| cells.iter().any(|c| c.is_string())).map(|(label, cells)| json!({ "label": label, "cells": cells })).collect();
    let mut note = if page == "every" { "Ectomatter sets these when a game starts and puts them back when the game quits.".to_string() } else { format!("Ectomatter sets these when {name} starts and puts them back when {name} quits.") };
    if several {
        note.push_str(" Turn off makes that monitor’s windows move to the others.");
    }
    let mut layout = vec![json!({ "grid": { "columns": columns, "rows": rows } })];
    if several {
        settings.insert("main".into(), menu(std::iter::once(leave()).chain(list.iter().map(|m| opt(m["id"].as_str().unwrap_or(""), &label(m)))).collect()));
        layout.push(json!({ "show": [{ "id": "main", "label": "Main monitor" }], "note": note }));
    } else {
        layout.push(json!({ "note": note }));
    }
    json!({ "section": { "label": "Monitor Switcher", "settings": settings, "layout": layout } })
}

/// One change: what it sets on which monitor, and to what, with the words the
/// keyboard's screen shows while it's made and the words for putting it back.
fn change(m: &Value, set: &str, to: Value, screen: &str, words: &str) -> Value {
    json!({ "change": { "monitor": m["id"], "set": set, "to": to }, "label": screen, "words": format!("the {}’s {words}", label(m)) })
}

/// What a game's choices change as it starts, in the order to make them: the
/// main monitor first, then each monitor's mode, HDR, variable refresh and
/// brightness, and last any turned off (never every one). A choice the
/// monitor is already at changes nothing.
fn changes(p: &Value) -> Result<Value, String> {
    let v = &p["values"];
    let list = monitors()?;
    let mut out = Vec::new();
    let get = |m: &Value, what: &str| -> Option<String> { v[format!("{}/{what}", m["id"].as_str().unwrap_or(""))].as_str().filter(|s| *s != "leave").map(String::from) };
    if let Some(main) = v["main"].as_str().filter(|s| *s != "leave") {
        if let Some(m) = list.iter().find(|m| m["id"] == main && m["main"] != true) {
            out.push(change(m, "main", json!(true), "MAIN MONITOR", "place as the main monitor"));
        }
    }
    let mut off = Vec::new();
    for m in &list {
        if get(m, "brightness").as_deref() == Some("off") {
            if m["enabled"] != false {
                off.push(m);
            }
            continue;
        }
        let (res, rate) = (get(m, "resolution"), get(m, "refresh"));
        if let (Some(now), true) = (current(m), res.is_some() || rate.is_some()) {
            let (w, h) = res.as_deref().and_then(|r| r.split_once('x')).and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))).unwrap_or((now.0, now.1));
            let want = rate.as_deref().and_then(|r| r.parse::<u64>().ok()).unwrap_or(hz(now.2));
            // that resolution at the rate nearest the one wanted
            let pick = m["modes"].as_array().into_iter().flatten().filter_map(mode_of).filter(|(mw, mh, _)| (*mw, *mh) == (w, h)).min_by_key(|(_, _, r)| (hz(*r).abs_diff(want), u64::MAX - r));
            if let Some(pick) = pick.filter(|p| *p != now) {
                let (screen, words) = if (pick.0, pick.1) != (now.0, now.1) { ("RESOLUTION", "resolution") } else { ("REFRESH RATE", "refresh rate") };
                out.push(change(m, "mode", json!({ "width": pick.0, "height": pick.1, "refresh": pick.2 }), screen, words));
            }
        }
        if let Some(on) = get(m, "hdr").map(|h| h == "on") {
            if m["hdr"].as_bool().is_some_and(|now| now != on) {
                out.push(change(m, "hdr", json!(on), "HDR", "HDR"));
            }
        }
        if let Some(vrr) = get(m, "vrr").map(|x| if x == "off" { "never".to_string() } else { x }) {
            if m["vrr"].as_str().is_some_and(|now| now != vrr) {
                out.push(change(m, "vrr", json!(vrr), "VARIABLE REFRESH", "variable refresh"));
            }
        }
        if let Some(b) = get(m, "brightness") {
            let level = match b.as_str() {
                "lowest" => Some(0),
                n => n.parse::<u64>().ok().map(|n| n.min(100) * 100),
            };
            if let (Some(level), Some(now)) = (level, m["brightness"].as_u64()) {
                if level != now {
                    out.push(change(m, "brightness", json!(level), "BRIGHTNESS", "brightness"));
                }
            }
        }
    }
    // (one monitor stays on: the main one, where every one is chosen to go off)
    let on = list.iter().filter(|m| m["enabled"] != false).count();
    if !off.is_empty() && off.len() >= on {
        off.retain(|m| m["main"] != true);
        off.truncate(on.saturating_sub(1));
    }
    for m in off {
        out.push(change(m, "enabled", json!(false), "MONITOR OFF", "power"));
    }
    Ok(json!({ "changes": out }))
}

fn answer(method: &str, p: &Value) -> Result<Value, String> {
    match method {
        "initialize" => ectomatter_protocol::meet(p, "Monitor Switcher", env!("CARGO_PKG_VERSION")),
        "game/describe" => Ok(describe(p)),
        "game/changes" => changes(p),
        "shutdown" => Ok(Value::Null),
        _ => Err(format!("The Monitor Switcher doesn’t answer {method}.")),
    }
}

struct Plugin;

impl Guest for Plugin {
    fn call(method: String, params: String) -> Result<String, String> {
        let p: Value = if params.is_empty() { Value::Null } else { serde_json::from_str(&params).map_err(|e| format!("{method}: {e}"))? };
        answer(&method, &p).map(|v| v.to_string())
    }

    fn timer(_tag: u32) {}
}

export!(Plugin);
