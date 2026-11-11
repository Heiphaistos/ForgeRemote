//! Table unique des touches. Colonnes : id, Samsung, LG, Panasonic, Roku, Philips, Sony.
//! Chaîne vide = touche absente de la marque (refusée proprement).
type Row = (&'static str, [&'static str; 6]);
const K: &[Row] = &[
    ("power", ["KEY_POWER", "POWER", "NRC_POWER-ONOFF", "Power", "Standby", "Power"]),
    ("0", ["KEY_0", "0", "NRC_D0-ONOFF", "Lit_0", "Digit0", "Num0"]),
    ("1", ["KEY_1", "1", "NRC_D1-ONOFF", "Lit_1", "Digit1", "Num1"]),
    ("2", ["KEY_2", "2", "NRC_D2-ONOFF", "Lit_2", "Digit2", "Num2"]),
    ("3", ["KEY_3", "3", "NRC_D3-ONOFF", "Lit_3", "Digit3", "Num3"]),
    ("4", ["KEY_4", "4", "NRC_D4-ONOFF", "Lit_4", "Digit4", "Num4"]),
    ("5", ["KEY_5", "5", "NRC_D5-ONOFF", "Lit_5", "Digit5", "Num5"]),
    ("6", ["KEY_6", "6", "NRC_D6-ONOFF", "Lit_6", "Digit6", "Num6"]),
    ("7", ["KEY_7", "7", "NRC_D7-ONOFF", "Lit_7", "Digit7", "Num7"]),
    ("8", ["KEY_8", "8", "NRC_D8-ONOFF", "Lit_8", "Digit8", "Num8"]),
    ("9", ["KEY_9", "9", "NRC_D9-ONOFF", "Lit_9", "Digit9", "Num9"]),
    ("up", ["KEY_UP", "UP", "NRC_UP-ONOFF", "Up", "CursorUp", "Up"]),
    ("down", ["KEY_DOWN", "DOWN", "NRC_DOWN-ONOFF", "Down", "CursorDown", "Down"]),
    ("left", ["KEY_LEFT", "LEFT", "NRC_LEFT-ONOFF", "Left", "CursorLeft", "Left"]),
    ("right", ["KEY_RIGHT", "RIGHT", "NRC_RIGHT-ONOFF", "Right", "CursorRight", "Right"]),
    ("ok", ["KEY_ENTER", "ENTER", "NRC_ENTER-ONOFF", "Select", "Confirm", "Confirm"]),
    ("back", ["KEY_RETURN", "BACK", "NRC_RETURN-ONOFF", "Back", "Back", "Return"]),
    ("home", ["KEY_HOME", "HOME", "NRC_HOME-ONOFF", "Home", "Home", "Home"]),
    ("menu", ["KEY_MENU", "MENU", "NRC_MENU-ONOFF", "", "Options", "Options"]),
    ("exit", ["KEY_EXIT", "EXIT", "NRC_CANCEL-ONOFF", "", "", "Exit"]),
    ("vol_up", ["KEY_VOLUP", "VOLUMEUP", "NRC_VOLUP-ONOFF", "VolumeUp", "VolumeUp", "VolumeUp"]),
    ("vol_down", ["KEY_VOLDOWN", "VOLUMEDOWN", "NRC_VOLDOWN-ONOFF", "VolumeDown", "VolumeDown", "VolumeDown"]),
    ("mute", ["KEY_MUTE", "MUTE", "NRC_MUTE-ONOFF", "VolumeMute", "Mute", "Mute"]),
    ("ch_up", ["KEY_CHUP", "CHANNELUP", "NRC_CH_UP-ONOFF", "ChannelUp", "ChannelStepUp", "ChannelUp"]),
    ("ch_down", ["KEY_CHDOWN", "CHANNELDOWN", "NRC_CH_DOWN-ONOFF", "ChannelDown", "ChannelStepDown", "ChannelDown"]),
    ("source", ["KEY_SOURCE", "", "NRC_CHG_INPUT-ONOFF", "", "Source", "Input"]),
    ("info", ["KEY_INFO", "INFO", "NRC_INFO-ONOFF", "Info", "Info", "Display"]),
    ("guide", ["KEY_GUIDE", "GUIDE", "NRC_EPG-ONOFF", "", "", "EPG"]),
    ("play", ["KEY_PLAY", "PLAY", "NRC_PLAY-ONOFF", "Play", "Play", "Play"]),
    ("pause", ["KEY_PAUSE", "PAUSE", "NRC_PAUSE-ONOFF", "Play", "Pause", "Pause"]),
    ("stop", ["KEY_STOP", "STOP", "NRC_STOP-ONOFF", "", "Stop", "Stop"]),
    ("rew", ["KEY_REWIND", "REWIND", "NRC_REW-ONOFF", "Rev", "Rewind", "Rewind"]),
    ("ff", ["KEY_FF", "FASTFORWARD", "NRC_FF-ONOFF", "Fwd", "FastForward", "Forward"]),
    ("red", ["KEY_RED", "RED", "NRC_RED-ONOFF", "", "RedColour", "Red"]),
    ("green", ["KEY_GREEN", "GREEN", "NRC_GREEN-ONOFF", "", "GreenColour", "Green"]),
    ("yellow", ["KEY_YELLOW", "YELLOW", "NRC_YELLOW-ONOFF", "", "YellowColour", "Yellow"]),
    ("blue", ["KEY_CYAN", "BLUE", "NRC_BLUE-ONOFF", "", "BlueColour", "Blue"]),
    // Touches supplémentaires : noms vérifiés dans les sources publiques (jaruba/ha-samsungtv-tizen Key_codes.md,
    // bscpylgtv buttons.py, panasonic-viera keys.py, doc Roku ECP). Vide = absent de la marque.
    ("tools", ["KEY_TOOLS", "QMENU", "NRC_SUBMENU-ONOFF", "", "", ""]),
    ("prev_ch", ["KEY_PRECH", "", "NRC_R_TUNE-ONOFF", "", "", ""]),
    ("ch_list", ["KEY_CH_LIST", "LIST", "", "", "", ""]),
    ("subtitle", ["KEY_SUB_TITLE", "CC", "NRC_STTL-ONOFF", "", "", ""]),
    ("teletext", ["KEY_TTX_MIX", "TELETEXT", "NRC_TEXT-ONOFF", "", "", ""]),
    ("aspect", ["KEY_PICTURE_SIZE", "ASPECT_RATIO", "NRC_ASPECT-ONOFF", "", "", ""]),
    ("sleep", ["KEY_SLEEP", "TIMER", "NRC_OFFTIMER-ONOFF", "", "", ""]),
    ("rec", ["KEY_REC", "RECORD", "NRC_REC-ONOFF", "", "", ""]),
    ("fav", ["KEY_FAVCH", "", "NRC_FAVORITE-ONOFF", "", "", ""]),
    ("audio_desc", ["KEY_AD", "AD", "", "", "", ""]),
    ("tv", ["KEY_TV", "TV", "NRC_TV-ONOFF", "", "", ""]),
    ("apps", ["KEY_APP_LIST", "MYAPPS", "NRC_APPS-ONOFF", "", "", ""]),
    ("netflix", ["", "NETFLIX", "NRC_NETFLIX-ONOFF", "", "", ""]),
    ("prime", ["", "AMAZON", "", "", "", ""]),
    ("search", ["", "SEARCH", "", "Search", "", ""]),
    ("replay", ["KEY_INSTANT_REPLAY", "", "", "InstantReplay", "", ""]),
    ("dash", ["KEY_PLUS100", "DASH", "", "", "", ""]),
    ("mode_3d", ["", "3D_MODE", "NRC_3D-ONOFF", "", "", ""]),
    ("power_on", ["KEY_POWERON", "", "", "PowerOn", "", ""]),
    ("power_off", ["KEY_POWEROFF", "", "", "PowerOff", "", ""]),
    ("hdmi", ["KEY_HDMI", "", "", "", "", ""]),
    ("hdmi1", ["", "", "NRC_HDMI1-ONOFF", "InputHDMI1", "", ""]),
    ("hdmi2", ["", "", "NRC_HDMI2-ONOFF", "InputHDMI2", "", ""]),
    ("hdmi3", ["", "", "NRC_HDMI3-ONOFF", "InputHDMI3", "", ""]),
    ("hdmi4", ["", "", "NRC_HDMI4-ONOFF", "InputHDMI4", "", ""]),
];

pub const BRANDS: [&str; 6] = ["samsung", "lg", "panasonic", "roku", "philips", "sony"];

/// Code de la touche pour une marque (index dans BRANDS).
pub fn code(id: &str, brand: usize) -> Result<&'static str, String> {
    let e = K.iter().find(|k| k.0 == id).ok_or(format!("touche inconnue : {id}"))?;
    match e.1.get(brand) {
        Some(c) if !c.is_empty() => Ok(c),
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
        assert!(code("menu", 3).is_err());
        assert!(code("nope", 0).is_err());
    }
}
