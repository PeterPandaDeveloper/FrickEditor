use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// Arquetipos de comportamiento soportados por el motor (Fase 1 + 2 auditadas).
/// Cada arquetipo define dónde aparece el animatrónico en la oficina y cómo
/// bloquea su avance (puerta, persiana, máscara, etc.).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimatronicArchetype {
    /// Ventana trasera con persianas (Murray Moose, golden_master_v26)
    BackWindowBlinds,
    /// Puerta del pasillo (se detiene con la puerta cerrada)
    HallwayDoorway,
    /// Sistema de ventilación (se desplaza por las rejillas)
    VentSystemCrawler,
    /// Ventana delantera con cristal
    FrontWindowGlass,
    /// Manifiesta en la propia oficina (aparición directa)
    OfficeManifest,
    /// Consola de la pared trasera (requiere tarea de consola)
    BackWallConsole,
}

impl Default for AnimatronicArchetype {
    fn default() -> Self {
        Self::BackWindowBlinds
    }
}

impl AnimatronicArchetype {
    pub fn all() -> [Self; 6] {
        [
            Self::BackWindowBlinds,
            Self::HallwayDoorway,
            Self::VentSystemCrawler,
            Self::FrontWindowGlass,
            Self::OfficeManifest,
            Self::BackWallConsole,
        ]
    }

    /// Punto (cx, cy) de aparición del animatrónico dentro de la oficina
    /// (coordenadas del ROI 640x360). `use_right_side` invierte el eje X.
    pub fn default_coordinates(&self, use_right_side: bool) -> (i32, i32) {
        let x = match self {
            Self::BackWindowBlinds => 116,
            Self::HallwayDoorway => 560,
            Self::VentSystemCrawler => 320,
            Self::FrontWindowGlass => 116,
            Self::OfficeManifest => 320,
            Self::BackWallConsole => 560,
        };
        let y = match self {
            Self::BackWindowBlinds => 96,
            Self::HallwayDoorway => 120,
            Self::VentSystemCrawler => 300,
            Self::FrontWindowGlass => 320,
            Self::OfficeManifest => 200,
            Self::BackWallConsole => 120,
        };
        if use_right_side {
            (640 - x, y)
        } else {
            (x, y)
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BackWindowBlinds => "Back Window (Blinds)",
            Self::HallwayDoorway => "Hallway (Doorway)",
            Self::VentSystemCrawler => "Vent System (Crawler)",
            Self::FrontWindowGlass => "Front Window (Glass)",
            Self::OfficeManifest => "Office (Manifest)",
            Self::BackWallConsole => "Back Wall (Console)",
        }
    }
}

/// Contra-medidas que derrotan a cada animatrónico. La condición exacta se
/// renderiza en el GML del `Step_0` según el arquetipo seleccionado.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountermeasureType {
    /// Quitar el accesorio (gafas) y después alumbrarlo
    RemoveAccessoryThenFlash,
    /// Alumbrarlo directamente con la linterna
    DirectFlashlight,
    /// NO alumbrarlo (se acerca si lo alumbras)
    DoNotFlashLight,
    /// Cerrar la puerta o la persiana
    CloseDoorOrBlind,
    /// Ponerse la máscara de Fredbear
    WearFreddyMask,
    /// Atraerlo con audio o electrocutarlo
    AudioLureOrShock,
    /// Resolver la tarea de la consola trasera
    SolveBackConsoleTask,
}

impl Default for CountermeasureType {
    fn default() -> Self {
        Self::RemoveAccessoryThenFlash
    }
}

impl CountermeasureType {
    pub fn all() -> [Self; 7] {
        [
            Self::RemoveAccessoryThenFlash,
            Self::DirectFlashlight,
            Self::DoNotFlashLight,
            Self::CloseDoorOrBlind,
            Self::WearFreddyMask,
            Self::AudioLureOrShock,
            Self::SolveBackConsoleTask,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::RemoveAccessoryThenFlash => "Remove Accessory + Flash",
            Self::DirectFlashlight => "Direct Flashlight",
            Self::DoNotFlashLight => "Do Not Flash Light",
            Self::CloseDoorOrBlind => "Close Door / Blind",
            Self::WearFreddyMask => "Wear Freddy Mask",
            Self::AudioLureOrShock => "Audio Lure / Shock",
            Self::SolveBackConsoleTask => "Solve Back Console Task",
        }
    }
}

/// Comportamiento de IA y animación del animatrónico (100% parametrizado por ID).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BehaviorConfig {
    pub archetype: AnimatronicArchetype,
    pub countermeasure: CountermeasureType,
    /// Si aparece en el lado derecho de la oficina (invierte cx/cy)
    pub use_right_side: bool,
    pub appear_in_four_layers_down: bool,
    pub four_layers_down_ai: i32,
    pub appear_in_endless_mode: bool,
    pub endless_mode_ai: i32,
    /// Frames que tarda en atacar con IA = 1 (60 FPS)
    pub attack_window_frames_ai1: i32,
    /// Frames que tarda en atacar con IA = 20 (60 FPS)
    pub attack_window_frames_ai20: i32,
    pub move_min_frames_ai1: i32,
    pub move_min_frames_ai20: i32,
    pub move_max_frames_ai1: i32,
    pub move_max_frames_ai20: i32,
    /// Offsets de dibujo de cada capa en el ROI de la oficina
    pub body_offset_x: i32,
    pub body_offset_y: i32,
    pub head_offset_x: i32,
    pub head_offset_y: i32,
    /// Offset del accesorio clicable (gafas) respecto a la cabeza
    pub accessory_offset_x: i32,
    pub accessory_offset_y: i32,
    /// Hitbox del accesorio clicable en GML
    pub accessory_hitbox_w: i32,
    pub accessory_hitbox_h: i32,
    /// Si se queda bloqueado cuando la puerta/persiana está cerrada
    pub stalls_when_blocked: bool,
    /// Si se puede comprar con Death Coin en el gestor de mods
    pub allow_death_coin: bool,
    /// Si abre la persiana al llegar (arquetipo BackWindowBlinds)
    pub opens_blind_on_arrival: bool,
    /// Cola de ventana sincronizada con Sun/Moon (evita multi-mod en la ventana)
    pub smart_queue_sun_moon: bool,
    /// Tokens que otorga al derrotarlo
    pub token_reward_on_defeat: i32,
    /// Overrides GML manuales: nombre de archivo -> código (reemplaza a los generados)
    pub custom_gml_overrides: BTreeMap<String, String>,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            archetype: AnimatronicArchetype::default(),
            countermeasure: CountermeasureType::default(),
            use_right_side: false,
            appear_in_four_layers_down: true,
            four_layers_down_ai: 12,
            appear_in_endless_mode: true,
            endless_mode_ai: 10,
            attack_window_frames_ai1: 600,
            attack_window_frames_ai20: 320,
            move_min_frames_ai1: 300,
            move_min_frames_ai20: 180,
            move_max_frames_ai1: 600,
            move_max_frames_ai20: 360,
            body_offset_x: 0,
            body_offset_y: 0,
            head_offset_x: 0,
            head_offset_y: 0,
            accessory_offset_x: 52,
            accessory_offset_y: 48,
            accessory_hitbox_w: 76,
            accessory_hitbox_h: 30,
            stalls_when_blocked: false,
            allow_death_coin: true,
            opens_blind_on_arrival: false,
            smart_queue_sun_moon: true,
            token_reward_on_defeat: 3,
            custom_gml_overrides: BTreeMap::new(),
        }
    }
}

/// Configuración de la Custom Night (Página de Mods) y los textos in-game
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomNightConfig {
    /// Índice del slot en la Página 2 de Custom Night (0 = primero arriba a la izquierda)
    pub slot_index: usize,
    /// Tooltip que aparece al pasar el ratón en Custom Night
    pub cursor_tooltip: String,
    /// Consejo en la pantalla de Game Over
    pub gameover_tip: String,
    /// Ubicación mostrada en el gestor de revisiones del menú de pausa
    pub pause_review_location: String,
    /// Descripción mostrada en el gestor de revisiones del menú de pausa
    pub pause_review_description: String,
}

impl Default for CustomNightConfig {
    fn default() -> Self {
        Self {
            slot_index: 0,
            cursor_tooltip: "Murray Moose: He knocks and opens the back-left window blind. Click directly on his sunglasses to snatch them off, then flash him while he is startled!".to_string(),
            gameover_tip: "Murray Moose: Click directly on his sunglasses to remove them, then flash him!".to_string(),
            pause_review_location: "Back-left window (office)".to_string(),
            pause_review_description: "Murray knocks 3 times before lifting the blind. Remove his glasses and flash him to earn tokens.".to_string(),
        }
    }
}

impl CustomNightConfig {
    /// Convierte el número de orden (Slot 0..31) en coordenadas exactas (tile_x, tile_y)
    /// de la Página 2 de Custom Night. La grilla nativa es de 8 columnas x 4 filas (32 slots),
    /// con paso de 54 px en X y 72 px en Y (ver golden_master_v26).
    pub fn slot_to_xy(&self) -> (i32, i32) {
        let col = (self.slot_index % 8) as i32;
        let row = (self.slot_index / 8) as i32;
        let x = 32 + (col * 54);
        let y = 12 + (row * 72);
        (x, y)
    }
}

/// Sprite extra opcional exportado como `s<ID>_<sprite_suffix>_0.png`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtraSprite {
    pub sprite_suffix: String,
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimatronicProject {
    pub internal_id: String,
    pub game_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub utmt_scripts_dir: PathBuf,
    pub gallery_info: ExtrasInfoJson,
    pub behavior: BehaviorConfig,
    pub custom_night: CustomNightConfig,

    // --- Galería y Custom Night Tile ---
    pub portrait_500_path: Option<PathBuf>,
    pub icon_48_path: Option<PathBuf>,
    pub auto_generate_gray_tile: bool,
    pub icon_gray_override_path: Option<PathBuf>,

    // --- Sprites de Oficina y Jumpscare ---
    pub body_sprite: Option<PathBuf>,
    pub head_normal_sprite: Option<PathBuf>,
    pub head_scare_sprite: Option<PathBuf>,
    pub head_gameover_alt: Option<PathBuf>,
    pub accessory_sprite: Option<PathBuf>,
    pub flash_defeat_sprite: Option<PathBuf>,
    pub extra_sprites: Vec<ExtraSprite>,
    pub jumpscare_frame_0: Option<PathBuf>,
    pub jumpscare_frame_1: Option<PathBuf>,
    pub jumpscare_frame_2: Option<PathBuf>,

    // --- Audios (.ogg) ---
    pub sound_knock_alert: Option<PathBuf>,
    pub sound_voice_line: Option<PathBuf>,
    pub sound_defeat: Option<PathBuf>,
    pub sound_jumpscare: Option<PathBuf>,

    // --- Mecánicas Avanzadas, Cámaras y Entornos 3D ---
    pub advanced: AdvancedMechanicsConfig,
}

/// Nodos de ruta en el mapa de cámaras (MainLocationMap)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CameraRouteNode {
    pub camera_id: u8,
    pub camera_name: String,
    pub sprite_path: Option<PathBuf>,
    pub transition_time_modifier: f32, // Multiplicador de velocidad para llegar a esta cámara
}

/// Objetos interactivos arrastrables en el Explorador 3D de Laberintos/Salvamento
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MazePropType {
    AnimatronicSpawn,   // oInteraction_Animatronic
    Hatchet,            // oMazeFloorDecoration_Hatchet
    GiftBox,            // oMazeProp_GiftBox
    Poster,             // oMazeSign_Poster
    ArcadeCabinet,      // oMazeProp_ArcadeCabinet
    EscapeDoor,         // oInteraction_Escape
    SalvageChair,       // oSalvage_Chair (Silla de Inspección / Salvamento)
    SalvageCassette,    // oSalvage_Cassette (Reproductor de audio de prueba)
    SalvageShock,       // oSalvage_Taser (Botón de electrochoque)
}

impl MazePropType {
    pub fn gml_object_name(&self) -> &'static str {
        match self {
            Self::AnimatronicSpawn => "oInteraction_Animatronic",
            Self::Hatchet => "oMazeFloorDecoration_Hatchet",
            Self::GiftBox => "oMazeProp_GiftBox",
            Self::Poster => "oMazeSign_Poster",
            Self::ArcadeCabinet => "oMazeProp_ArcadeCabinet",
            Self::EscapeDoor => "oInteraction_Escape",
            Self::SalvageChair => "oSalvageHead",
            Self::SalvageCassette => "oSalvageOpener_Salvage",
            Self::SalvageShock => "oSalvageSheet",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentEntity {
    pub prop_type: MazePropType,
    pub x: i32,
    pub y: i32,
    pub scale_x: f32,
    pub scale_y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdvancedMechanicsConfig {
    pub use_node_editor: bool, // Híbrido: True = Nodos Visuales, False = Código GML Manual
    pub camera_route: Vec<CameraRouteNode>,
    pub maze_entities: Vec<EnvironmentEntity>,
    pub target_maze_room: String,
}

impl Default for AdvancedMechanicsConfig {
    fn default() -> Self {
        Self {
            use_node_editor: true,
            camera_route: Vec::new(),
            maze_entities: Vec::new(),
            target_maze_room: "MinigameMaze_1".to_string(),
        }
    }
}

impl Default for AnimatronicProject {
    fn default() -> Self {
        let base_assets = if PathBuf::from("assets/sample_murray").exists() {
            PathBuf::from("assets/sample_murray")
        } else if PathBuf::from(r"D:\Frickbears-mod\Murray_Assets").exists() {
            PathBuf::from(r"D:\Frickbears-mod\Murray_Assets")
        } else {
            PathBuf::from("assets/sample_murray")
        };
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
            behavior: BehaviorConfig::default(),
            custom_night: CustomNightConfig::default(),
            portrait_500_path: auto_file("FB_Murray_Moose.png"),
            icon_48_path: auto_file("sTile_Murray_0.png"),
            auto_generate_gray_tile: true,
            icon_gray_override_path: None,
            body_sprite: auto_file("sMurray_body.png"),
            head_normal_sprite: auto_file("sMurray_head.png"),
            head_scare_sprite: auto_file("sMurray_head_Scare.png"),
            head_gameover_alt: auto_file("sMurray_head_Alt.png"),
            accessory_sprite: auto_file("sMurray_Glasses.png"),
            flash_defeat_sprite: auto_file("sMurray_Flash.png"),
            extra_sprites: Vec::new(),
            jumpscare_frame_0: auto_file("sJumpscare_Murray_0.png"),
            jumpscare_frame_1: auto_file("sJumpscare_Murray_1.png"),
            jumpscare_frame_2: auto_file("sJumpscare_Murray_2.png"),
            sound_knock_alert: auto_file("murray_knock.ogg"),
            sound_voice_line: auto_file("murray_amigable.ogg"),
            sound_defeat: auto_file("murray_knock2.ogg"),
            sound_jumpscare: auto_file("murray_jumpscare.ogg"),
            advanced: AdvancedMechanicsConfig::default(),
        }
    }
}

impl AnimatronicProject {
    pub fn readiness_score(&self) -> (usize, usize) {
        let checks: Vec<&Option<PathBuf>> = vec![
            &self.portrait_500_path,
            &self.icon_48_path,
            &self.body_sprite,
            &self.head_normal_sprite,
            &self.head_scare_sprite,
            &self.head_gameover_alt,
            &self.accessory_sprite,
            &self.flash_defeat_sprite,
            &self.jumpscare_frame_0,
            &self.jumpscare_frame_1,
            &self.jumpscare_frame_2,
            &self.sound_knock_alert,
            &self.sound_voice_line,
            &self.sound_defeat,
            &self.sound_jumpscare,
        ];
        let mut total = checks.len();
        let mut ready = checks
            .iter()
            .filter(|opt| opt.as_ref().map(|p| p.exists()).unwrap_or(false))
            .count();
        for extra in &self.extra_sprites {
            total += 1;
            if extra.path.exists() {
                ready += 1;
            }
        }
        (ready, total)
    }

    pub fn count_verified_assets(&self) -> usize {
        self.readiness_score().0
    }
}