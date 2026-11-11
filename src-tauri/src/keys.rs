//! Table unique des touches. Colonnes : id, Samsung, LG, Panasonic, Roku, Philips, Sony, Vizio, Xiaomi, LG Netcast, Fire TV (les 4 dernières : expérimentales, voir extra.rs).
//! Chaîne vide = touche absente de la marque (refusée proprement).
type Row = (&'static str, [&'static str; 10]);
const K: &[Row] = &[
    ("power", ["KEY_POWER", "POWER", "NRC_POWER-ONOFF", "Power", "Standby", "Power", "11:2", "power", "1", "sleep"]),
    ("0", ["KEY_0", "0", "NRC_D0-ONOFF", "Lit_0", "Digit0", "Num0", "", "", "2", "text:0"]),
    ("1", ["KEY_1", "1", "NRC_D1-ONOFF", "Lit_1", "Digit1", "Num1", "", "", "3", "text:1"]),
    ("2", ["KEY_2", "2", "NRC_D2-ONOFF", "Lit_2", "Digit2", "Num2", "", "", "4", "text:2"]),
    ("3", ["KEY_3", "3", "NRC_D3-ONOFF", "Lit_3", "Digit3", "Num3", "", "", "5", "text:3"]),
    ("4", ["KEY_4", "4", "NRC_D4-ONOFF", "Lit_4", "Digit4", "Num4", "", "", "6", "text:4"]),
    ("5", ["KEY_5", "5", "NRC_D5-ONOFF", "Lit_5", "Digit5", "Num5", "", "", "7", "text:5"]),
    ("6", ["KEY_6", "6", "NRC_D6-ONOFF", "Lit_6", "Digit6", "Num6", "", "", "8", "text:6"]),
    ("7", ["KEY_7", "7", "NRC_D7-ONOFF", "Lit_7", "Digit7", "Num7", "", "", "9", "text:7"]),
    ("8", ["KEY_8", "8", "NRC_D8-ONOFF", "Lit_8", "Digit8", "Num8", "", "", "10", "text:8"]),
    ("9", ["KEY_9", "9", "NRC_D9-ONOFF", "Lit_9", "Digit9", "Num9", "", "", "11", "text:9"]),
    ("up", ["KEY_UP", "UP", "NRC_UP-ONOFF", "Up", "CursorUp", "Up", "3:8", "up", "12", "dpad_up"]),
    ("down", ["KEY_DOWN", "DOWN", "NRC_DOWN-ONOFF", "Down", "CursorDown", "Down", "3:0", "down", "13", "dpad_down"]),
    ("left", ["KEY_LEFT", "LEFT", "NRC_LEFT-ONOFF", "Left", "CursorLeft", "Left", "3:1", "left", "14", "dpad_left"]),
    ("right", ["KEY_RIGHT", "RIGHT", "NRC_RIGHT-ONOFF", "Right", "CursorRight", "Right", "3:7", "right", "15", "dpad_right"]),
    ("ok", ["KEY_ENTER", "ENTER", "NRC_ENTER-ONOFF", "Select", "Confirm", "Confirm", "3:2", "enter", "20", "select"]),
    ("back", ["KEY_RETURN", "BACK", "NRC_RETURN-ONOFF", "Back", "Back", "Return", "4:0", "back", "23", "back"]),
    ("home", ["KEY_HOME", "HOME", "NRC_HOME-ONOFF", "Home", "Home", "Home", "4:15", "home", "21", "home"]),
    ("menu", ["KEY_MENU", "MENU", "NRC_MENU-ONOFF", "", "Options", "Options", "4:8", "menu", "22", "menu"]),
    ("exit", ["KEY_EXIT", "EXIT", "NRC_CANCEL-ONOFF", "", "", "Exit", "9:0", "", "412", ""]),
    ("vol_up", ["KEY_VOLUP", "VOLUMEUP", "NRC_VOLUP-ONOFF", "VolumeUp", "VolumeUp", "VolumeUp", "5:1", "volumeup", "24", "volume_up"]),
    ("vol_down", ["KEY_VOLDOWN", "VOLUMEDOWN", "NRC_VOLDOWN-ONOFF", "VolumeDown", "VolumeDown", "VolumeDown", "5:0", "volumedown", "25", "volume_down"]),
    ("mute", ["KEY_MUTE", "MUTE", "NRC_MUTE-ONOFF", "VolumeMute", "Mute", "Mute", "5:4", "", "26", "mute"]),
    ("ch_up", ["KEY_CHUP", "CHANNELUP", "NRC_CH_UP-ONOFF", "ChannelUp", "ChannelStepUp", "ChannelUp", "8:1", "", "27", ""]),
    ("ch_down", ["KEY_CHDOWN", "CHANNELDOWN", "NRC_CH_DOWN-ONOFF", "ChannelDown", "ChannelStepDown", "ChannelDown", "8:0", "", "28", ""]),
    ("source", ["KEY_SOURCE", "", "NRC_CHG_INPUT-ONOFF", "", "Source", "Input", "7:1", "", "47", ""]),
    ("info", ["KEY_INFO", "INFO", "NRC_INFO-ONOFF", "Info", "Info", "Display", "4:6", "", "45", ""]),
    ("guide", ["KEY_GUIDE", "GUIDE", "NRC_EPG-ONOFF", "", "", "EPG", "", "", "44", ""]),
    ("play", ["KEY_PLAY", "PLAY", "NRC_PLAY-ONOFF", "Play", "Play", "Play", "2:3", "", "33", "media:play"]),
    ("pause", ["KEY_PAUSE", "PAUSE", "NRC_PAUSE-ONOFF", "Play", "Pause", "Pause", "2:2", "", "34", "media:pause"]),
    ("stop", ["KEY_STOP", "STOP", "NRC_STOP-ONOFF", "", "Stop", "Stop", "", "", "35", ""]),
    ("rew", ["KEY_REWIND", "REWIND", "NRC_REW-ONOFF", "Rev", "Rewind", "Rewind", "2:1", "", "37", "scan:back"]),
    ("ff", ["KEY_FF", "FASTFORWARD", "NRC_FF-ONOFF", "Fwd", "FastForward", "Forward", "2:0", "", "36", "scan:forward"]),
    ("red", ["KEY_RED", "RED", "NRC_RED-ONOFF", "", "RedColour", "Red", "", "", "31", ""]),
    ("green", ["KEY_GREEN", "GREEN", "NRC_GREEN-ONOFF", "", "GreenColour", "Green", "", "", "30", ""]),
    ("yellow", ["KEY_YELLOW", "YELLOW", "NRC_YELLOW-ONOFF", "", "YellowColour", "Yellow", "", "", "32", ""]),
    ("blue", ["KEY_CYAN", "BLUE", "NRC_BLUE-ONOFF", "", "BlueColour", "Blue", "", "", "29", ""]),
    // Touches supplémentaires : noms vérifiés dans les sources publiques (jaruba/ha-samsungtv-tizen Key_codes.md,
    // bscpylgtv buttons.py, panasonic-viera keys.py, doc Roku ECP). Vide = absent de la marque.
    ("tools", ["KEY_TOOLS", "QMENU", "NRC_SUBMENU-ONOFF", "", "", "", "", "", "405", ""]),
    ("prev_ch", ["KEY_PRECH", "", "NRC_R_TUNE-ONOFF", "", "", "", "8:2", "", "403", ""]),
    ("ch_list", ["KEY_CH_LIST", "LIST", "", "", "", "", "", "", "50", ""]),
    ("subtitle", ["KEY_SUB_TITLE", "CC", "NRC_STTL-ONOFF", "", "", "", "4:4", "", "49", ""]),
    ("teletext", ["KEY_TTX_MIX", "TELETEXT", "NRC_TEXT-ONOFF", "", "", "", "", "", "51", ""]),
    ("aspect", ["KEY_PICTURE_SIZE", "ASPECT_RATIO", "NRC_ASPECT-ONOFF", "", "", "", "", "", "46", ""]),
    ("sleep", ["KEY_SLEEP", "TIMER", "NRC_OFFTIMER-ONOFF", "", "", "", "", "", "", ""]),
    ("rec", ["KEY_REC", "RECORD", "NRC_REC-ONOFF", "", "", "", "", "", "40", ""]),
    ("fav", ["KEY_FAVCH", "", "NRC_FAVORITE-ONOFF", "", "", "", "", "", "404", ""]),
    ("audio_desc", ["KEY_AD", "AD", "", "", "", "", "", "", "407", ""]),
    ("tv", ["KEY_TV", "TV", "NRC_TV-ONOFF", "", "", "", "", "", "", ""]),
    ("apps", ["KEY_APP_LIST", "MYAPPS", "NRC_APPS-ONOFF", "", "", "", "", "", "417", ""]),
    ("netflix", ["APP:netflix", "NETFLIX", "NRC_NETFLIX-ONOFF", "APP:netflix", "", "", "", "", "", ""]),
    ("prime", ["APP:prime video", "AMAZON", "", "APP:prime video", "", "", "", "", "", ""]),
    // « APP:nom » = lancer l'application de ce nom, retrouvée dans la liste d'applis de la TV (pas d'identifiant deviné).
    ("plex", ["APP:plex", "APP:plex", "", "APP:plex", "", "", "", "", "", ""]),
    ("youtube", ["APP:youtube", "APP:youtube", "", "APP:youtube", "", "", "", "", "", ""]),
    ("disney", ["APP:disney", "APP:disney", "", "APP:disney", "", "", "", "", "", ""]),
    ("search", ["", "SEARCH", "", "Search", "", "", "", "", "", ""]),
    ("replay", ["KEY_INSTANT_REPLAY", "", "", "InstantReplay", "", "", "", "", "", ""]),
    ("dash", ["KEY_PLUS100", "DASH", "", "", "", "", "", "", "", ""]),
    ("mode_3d", ["", "3D_MODE", "NRC_3D-ONOFF", "", "", "", "", "", "400", ""]),
    ("power_on", ["KEY_POWERON", "", "", "PowerOn", "", "", "11:1", "", "", ""]),
    ("power_off", ["KEY_POWEROFF", "", "", "PowerOff", "", "", "11:0", "", "", ""]),
    ("hdmi", ["KEY_HDMI", "", "", "", "", "", "", "", "", ""]),
    ("hdmi1", ["", "", "NRC_HDMI1-ONOFF", "InputHDMI1", "", "", "", "", "", ""]),
    ("hdmi2", ["", "", "NRC_HDMI2-ONOFF", "InputHDMI2", "", "", "", "", "", ""]),
    ("hdmi3", ["", "", "NRC_HDMI3-ONOFF", "InputHDMI3", "", "", "", "", "", ""]),
    ("hdmi4", ["", "", "NRC_HDMI4-ONOFF", "InputHDMI4", "", "", "", "", "", ""]),
];

pub const BRANDS: [&str; 12] = ["samsung", "lg", "panasonic", "roku", "philips", "sony", "vizio", "xiaomi", "lgnetcast", "firetv", "androidtv", "samsunglegacy"];

/// Identifiants de touches gérés par une marque Wi-Fi (index dans BRANDS).
pub fn supported(brand: usize) -> Vec<&'static str> {
    if brand == 11 { return supported(0).into_iter().filter(|i| !i.starts_with("hdmi")).collect(); } // mêmes KEY_ que Samsung
    if brand == 10 { return K.iter().map(|k| k.0).filter(|id| crate::androidtv::touche_android(id).is_some()).collect(); }
    K.iter().filter(|k| k.1.get(brand).is_some_and(|c| !c.is_empty())).map(|k| k.0).collect()
}

/// Code de la touche pour une marque (index dans BRANDS). « app:nom » = raccourci d'application libre :
/// lancée par son nom (liste d'applis de la TV) sur Samsung, LG et Roku seulement.
pub fn code(id: &str, brand: usize) -> Result<String, String> {
    if let Some(nom) = id.strip_prefix("app:") {
        let nom = nom.trim().to_lowercase();
        if nom.is_empty() || nom.chars().count() > 40 || !nom.chars().all(|c| c.is_alphanumeric() || " +-.&'".contains(c)) {
            return Err("nom d'application invalide".into());
        }
        return if matches!(brand, 0 | 1 | 3 | 10) { Ok(format!("APP:{nom}")) } else { Err("lancement d'applications non géré par cette marque".into()) };
    }
    if let Some(s) = id.strip_prefix("src:") {
        // Entrée de la TV choisie dans sa liste (LG webOS : tv/switchInput) : identifiant alphanumérique seulement.
        return if brand == 1 && !s.is_empty() && s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { Ok(format!("SRC:{s}")) } else { Err("entrée non gérée par cette marque".into()) };
    }
    if brand == 10 { return crate::androidtv::touche_android(id).map(|c| format!("KEY:{c}")).ok_or_else(|| format!("touche « {id} » non gérée par Android TV")); }
    let brand = if brand == 11 { 0 } else { brand };
    let e = K.iter().find(|k| k.0 == id).ok_or(format!("touche inconnue : {id}"))?;
    match e.1.get(brand) {
        Some(c) if !c.is_empty() => Ok(c.to_string()),
        _ => Err(format!("touche « {id} » non gérée par cette marque")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samsung_complet_et_vides_refuses() {
        assert!(K.iter().take(37).all(|k| !k.1[0].is_empty()));
        assert!(code("tools", 0).is_ok() && code("tools", 3).is_err() && code("hdmi2", 3).unwrap() == "InputHDMI2");
        assert_eq!(code("vol_up", 3).unwrap(), "VolumeUp");
        assert!(supported(0).contains(&"tools") && !supported(3).contains(&"tools") && supported(3).contains(&"hdmi2"));
        assert_eq!(code("app:Disney+", 0).unwrap(), "APP:disney+");
        assert!(code("app:", 0).is_err() && code("app:x/y", 0).is_err() && code("app:plex", 2).is_err());
        assert!(code("menu", 3).is_err());
        assert!(code("nope", 0).is_err());
        assert_eq!(code("src:HDMI_2", 1).unwrap(), "SRC:HDMI_2");
        assert!(code("src:HDMI_2", 0).is_err() && code("src:a b", 1).is_err() && code("src:", 1).is_err());
        assert_eq!(code("vol_up", 6).unwrap(), "5:1");
        assert_eq!(code("vol_up", 7).unwrap(), "volumeup");
        assert_eq!(code("5", 8).unwrap(), "7");
        assert_eq!(code("ok", 9).unwrap(), "select");
        assert!(supported(8).len() > 30 && supported(9).len() >= 8);
    }
}
