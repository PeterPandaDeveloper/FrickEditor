use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Estructura exacta que lee Frickbear's 3 en `addons/<Nombre>/extras_info.txt`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtrasInfoJson {
    #[serde(rename = "FULL_NAME")]
    pub full_name: String,
    #[serde(rename = "DEBUT")]
    pub debut: String,
    #[serde(rename = "DESCRIPTION")]
    pub description: String,
    #[serde(rename = "DEV_NOTES")]
    pub dev_notes: String,
    #[serde(rename = "VACredit")]
    pub va_credit: String,
}

impl Default for ExtrasInfoJson {
    fn default() -> Self {
        Self {
            full_name: "Murray Moose".to_string(),
            debut: "2025".to_string(),
            description: "Created as one of the prototype models for a new form of Advanced IA, Murray was meant to be the star of the new wave of animatronic performers made by Voltur-enterprise.".to_string(),
            dev_notes: "Created by Ghostly Candela & Peter Z.".to_string(),
            va_credit: "Peter Z".to_string(),
        }
    }
}

/// Estado completo del Módulo 1 (Galería + Sprites + Audios)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimatronicProject {
    pub internal_id: String,
    pub game_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub gallery_info: ExtrasInfoJson,

    // --- Imágenes de Galería y Custom Night ---
    pub portrait_500_path: Option<PathBuf>,  // Se exporta a 500x500 como portrait.png
    pub icon_48_path: Option<PathBuf>,       // Se exporta a 48x48 como icon.png y sTile_<ID>_0.png
    pub auto_generate_gray_tile: bool,       // Genera automáticamente sTile_<ID>_1.png en escala de grises

    // --- Sprites de Oficina y Jumpscare ---
    pub body_sprite: Option<PathBuf>,
    pub head_normal_sprite: Option<PathBuf>,
    pub head_scare_sprite: Option<PathBuf>,
    pub head_gameover_alt: Option<PathBuf>,
    pub accessory_glasses_sprite: Option<PathBuf>,
    pub flash_defeat_sprite: Option<PathBuf>,
    pub jumpscare_frame_0: Option<PathBuf>,
    pub jumpscare_frame_1: Option<PathBuf>,
    pub jumpscare_frame_2: Option<PathBuf>,

    // --- Archivos de Audio (.ogg) ---
    pub sound_knock: Option<PathBuf>,
    pub sound_voice_arrive: Option<PathBuf>,
    pub sound_defeat: Option<PathBuf>,
}

impl Default for AnimatronicProject {
    fn default() -> Self {
        Self {
            internal_id: "Murray".to_string(),
            game_dir: PathBuf::from(r"C:\Juegos\frickbears3-4ld-v2.0.2"),
            workspace_dir: PathBuf::from(r"D:\Frickbears-mod"),
            gallery_info: ExtrasInfoJson::default(),
            portrait_500_path: None,
            icon_48_path: None,
            auto_generate_gray_tile: true,
            body_sprite: None,
            head_normal_sprite: None,
            head_scare_sprite: None,
            head_gameover_alt: None,
            accessory_glasses_sprite: None,
            flash_defeat_sprite: None,
            jumpscare_frame_0: None,
            jumpscare_frame_1: None,
            jumpscare_frame_2: None,
            sound_knock: None,
            sound_voice_arrive: None,
            sound_defeat: None,
        }
    }
}
