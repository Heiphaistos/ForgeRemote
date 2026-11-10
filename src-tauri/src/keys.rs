//! Table unique des touches : (id interne, Samsung, LG pointeur, Panasonic).
//! Une touche absente d'une marque = chaîne vide (refusée proprement).
pub const KEYS: &[(&str, &str, &str, &str)] = &[
    ("power", "KEY_POWER", "POWER", "NRC_POWER-ONOFF"),
    ("0", "KEY_0", "0", "NRC_D0-ONOFF"),
    ("1", "KEY_1", "1", "NRC_D1-ONOFF"),
    ("2", "KEY_2", "2", "NRC_D2-ONOFF"),
    ("3", "KEY_3", "3", "NRC_D3-ONOFF"),
    ("4", "KEY_4", "4", "NRC_D4-ONOFF"),
    ("5", "KEY_5", "5", "NRC_D5-ONOFF"),
    ("6", "KEY_6", "6", "NRC_D6-ONOFF"),
    ("7", "KEY_7", "7", "NRC_D7-ONOFF"),
    ("8", "KEY_8", "8", "NRC_D8-ONOFF"),
    ("9", "KEY_9", "9", "NRC_D9-ONOFF"),
    ("up", "KEY_UP", "UP", "NRC_UP-ONOFF"),
    ("down", "KEY_DOWN", "DOWN", "NRC_DOWN-ONOFF"),
    ("left", "KEY_LEFT", "LEFT", "NRC_LEFT-ONOFF"),
    ("right", "KEY_RIGHT", "RIGHT", "NRC_RIGHT-ONOFF"),
    ("ok", "KEY_ENTER", "ENTER", "NRC_ENTER-ONOFF"),
    ("back", "KEY_RETURN", "BACK", "NRC_RETURN-ONOFF"),
    ("home", "KEY_HOME", "HOME", "NRC_HOME-ONOFF"),
    ("menu", "KEY_MENU", "MENU", "NRC_MENU-ONOFF"),
    ("exit", "KEY_EXIT", "EXIT", "NRC_CANCEL-ONOFF"),
    ("vol_up", "KEY_VOLUP", "VOLUMEUP", "NRC_VOLUP-ONOFF"),
    ("vol_down", "KEY_VOLDOWN", "VOLUMEDOWN", "NRC_VOLDOWN-ONOFF"),
    ("mute", "KEY_MUTE", "MUTE", "NRC_MUTE-ONOFF"),
    ("ch_up", "KEY_CHUP", "CHANNELUP", "NRC_CH_UP-ONOFF"),
    ("ch_down", "KEY_CHDOWN", "CHANNELDOWN", "NRC_CH_DOWN-ONOFF"),
    ("source", "KEY_SOURCE", "", "NRC_CHG_INPUT-ONOFF"),
    ("info", "KEY_INFO", "INFO", "NRC_INFO-ONOFF"),
    ("guide", "KEY_GUIDE", "GUIDE", "NRC_EPG-ONOFF"),
    ("play", "KEY_PLAY", "PLAY", "NRC_PLAY-ONOFF"),
    ("pause", "KEY_PAUSE", "PAUSE", "NRC_PAUSE-ONOFF"),
    ("stop", "KEY_STOP", "STOP", "NRC_STOP-ONOFF"),
    ("rew", "KEY_REWIND", "REWIND", "NRC_REW-ONOFF"),
    ("ff", "KEY_FF", "FASTFORWARD", "NRC_FF-ONOFF"),
    ("red", "KEY_RED", "RED", "NRC_RED-ONOFF"),
    ("green", "KEY_GREEN", "GREEN", "NRC_GREEN-ONOFF"),
    ("yellow", "KEY_YELLOW", "YELLOW", "NRC_YELLOW-ONOFF"),
    ("blue", "KEY_CYAN", "BLUE", "NRC_BLUE-ONOFF"),
];

/// brand: 1=Samsung, 2=LG, 3=Panasonic. Renvoie le code de la marque ou une erreur.
pub fn code(id: &str, brand: u8) -> Result<&'static str, String> {
    let e = KEYS.iter().find(|k| k.0 == id).ok_or(format!("touche inconnue : {id}"))?;
    let c = match brand { 1 => e.1, 2 => e.2, _ => e.3 };
    if c.is_empty() { Err(format!("touche « {id} » non gérée par cette marque")) } else { Ok(c) }
}
