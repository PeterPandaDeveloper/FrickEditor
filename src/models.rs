use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
            description: "Created as one of the prototype models for a new form of Advanced IA, Murray was meant to be the star of the new wave of animatronic performers made by Voltur-enterprise.\n\nHe, of course, won't accept that kind of fate! His career has yet to start, and he'll make the most out of it!\n\nJust... Don't tell him Disco music isn't really popular anymore.".to_string(),
            dev_notes: "Murray was originally created to be the face of a FNaF coop map for Minecraft! I asked my best friend to give me two random topics and he said \"Disco and Moose\". That's all I needed to create our Freddy Fazbear!".to_string(),
            va_credit: "Peter Z".to_string(),
        }
    }
}

/// Configuración de orden en la Página de Mods (Custom Night) y balanceo de IA
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MechanicsAndGridConfig {
    /// Índice del slot en la Página 2 de Custom Night (0 = primero arriba a la izquierda)
    pub custom_night_slot_index: usize,
    /// Tooltip que aparece al pasar el ratón en Custom Night
    pub custom_night_tooltip: String,
    /// Consejo en la pantalla de Game Over
    pub gameover_tip: String,
    /// Nivel de IA automático en modo Endless (1..20)
    pub endless_default_ai: u8,
    /// Nivel de IA automático en campaña Four Layers Down (1..20)
    pub four_layers_down_ai: u8,
    /// Segundos base para atacar con IA = 1 vs IA = 20
    pub attack_frames_ai1: i32,
    pub attack_frames_ai20: i32,
    /// Segundos mínimos y máximos para tocar la ventana con IA = 20
    pub knock_min_frames_ai20: i32,
    pub knock_max_frames_ai20: i32,
    /// Offset del accesorio clicable (gafas) respecto a la cabeza
    pub glasses_offset_x: i32,
    pub glasses_offset_y: i32,
}

impl Default for MechanicsAndGridConfig {
    fn default() -> Self {
        Self {
            custom_night_slot_index: 0,
            custom_night_tooltip: "Murray Moose: He knocks and opens the back-left window blind. Click directly on his sunglasses to snatch them off, then flash him while he is startled!".to_string(),
            gameover_tip: "Murray Moose: Click directly on his sunglasses to remove them, then flash him!".to_string(),
            endless_default_ai: 10,
            four_layers_down_ai: 12,
            attack_frames_ai1: 600,  // 10.0s a 60 FPS
            attack_frames_ai20: 320, // 5.3s a 60 FPS
            knock_min_frames_ai20: 180,
            knock_max_frames_ai20: 360,
            glasses_offset_x: 52,
            glasses_offset_y: 48,
        }
    }
}

impl MechanicsAndGridConfig {
    /// Convierte el número de orden (Slot 0..49) en coordenadas exactas (tile_x, tile_y) de la grilla de Frickbear's
    pub fn slot_to_xy(&self) -> (i32, i32) {
        let col = (self.custom_night_slot_index % 10) as i32;
        let row = (self.custom_night_slot_index / 10) as i32;
        let x = 32 + (col * 44);
        let y = 12 + (row * 68);
        (x, y)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimatronicProject {
    pub internal_id: String,
    pub game_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub utmt_scripts_dir: PathBuf,
    pub gallery_info: ExtrasInfoJson,
    pub mechanics: MechanicsAndGridConfig,

    // --- Galería y Custom Night Tile ---
    pub portrait_500_path: Option<PathBuf>,
    pub icon_48_path: Option<PathBuf>,
    pub auto_generate_gray_tile: bool,

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

    // --- Audios (.ogg) ---
    pub sound_knock: Option<PathBuf>,
    pub sound_voice_arrive: Option<PathBuf>,
    pub sound_defeat: Option<PathBuf>,
}

impl Default for AnimatronicProject {
    fn default() -> Self {
        let base_assets = PathBuf::from(r"D:\Frickbears-mod\Murray_Assets");
        let auto_file = |name: &str| -> Option<PathBuf> {
            let p = base_assets.join(name);
            if p.exists() { Some(p) } else { None }
        };

        Self {
            internal_id: "Murray".to_string(),
            game_dir: PathBuf::from(r"C:\Juegos\frickbears3-4ld-v2.0.2"),
            workspace_dir: PathBuf::from(r"D:\Frickbears-mod"),
            utmt_scripts_dir: PathBuf::from(r"D:\UndertaleModTool_v0.9.2.0-Windows-SingleFile\Scripts"),
            gallery_info: ExtrasInfoJson::default(),
            mechanics: MechanicsAndGridConfig::default(),
            portrait_500_path: auto_file("FB_Murray_Moose.png"),
            icon_48_path: auto_file("sTile_Murray_0.png"),
            auto_generate_gray_tile: true,
            body_sprite: auto_file("sMurray_body.png"),
            head_normal_sprite: auto_file("sMurray_head.png"),
            head_scare_sprite: auto_file("sMurray_head_Scare.png"),
            head_gameover_alt: auto_file("sMurray_head_Alt.png"),
            accessory_glasses_sprite: auto_file("sMurray_Glasses.png"),
            flash_defeat_sprite: auto_file("sMurray_Flash.png"),
            jumpscare_frame_0: auto_file("sJumpscare_Murray_0.png"),
            jumpscare_frame_1: auto_file("sJumpscare_Murray_1.png"),
            jumpscare_frame_2: auto_file("sJumpscare_Murray_2.png"),
            sound_knock: auto_file("murray_knock.ogg"),
            sound_voice_arrive: auto_file("murray_amigable.ogg"),
            sound_defeat: auto_file("murray_knock2.ogg"),
        }
    }
}

impl AnimatronicProject {
    pub fn readiness_score(&self) -> (usize, usize) {
        let checks = [
            &self.portrait_500_path,
            &self.icon_48_path,
            &self.body_sprite,
            &self.head_normal_sprite,
            &self.head_scare_sprite,
            &self.head_gameover_alt,
            &self.accessory_glasses_sprite,
            &self.flash_defeat_sprite,
            &self.jumpscare_frame_0,
            &self.jumpscare_frame_1,
            &self.jumpscare_frame_2,
            &self.sound_knock,
            &self.sound_voice_arrive,
            &self.sound_defeat,
        ];
        let total = checks.len();
        let ready = checks
            .iter()
            .filter(|opt| opt.as_ref().map(|p| p.exists()).unwrap_or(false))
            .count();
        (ready, total)
    }
}
