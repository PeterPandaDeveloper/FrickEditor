mod builder;
mod launcher;
mod mod_manager;
mod models;
pub mod node_editor;

use builder::IdeBuilder;
use launcher::GameLauncher;
use mod_manager::ModManager;
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, RichText, Rounding, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use egui_phosphor::regular as ic;
use models::{AnimatronicArchetype, AnimatronicProject, CameraRouteNode, CountermeasureType, MazeRoomData, SalvageLocationIndex};
use rodio::{Decoder, OutputStream, Sink};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([940.0, 620.0])
            .with_title("FrickEditor Studio — Suite Completa de Animatrónicos"),
        ..Default::default()
    };
    eframe::run_native(
        "FrickEditor",
        options,
        Box::new(|cc| Ok(Box::new(FrickEditorApp::new(cc)))),
    )
}

#[derive(PartialEq, Clone, Copy)]
enum ActiveTab {
    ModManager,
    Gallery,
    SpritesAndGuide,
    LiveAnimationPreview,
    GridAndMechanics,
    CameraMapRouter,
    Environment3DBuilder,
}

#[derive(PartialEq, Clone, Copy)]
enum GridSubView {
    CustomNightGrid,
    FourLayersDownSelector,
    PauseMenuPreview,
}

#[derive(PartialEq, Clone, Copy)]
enum SalvageTabMode {
    LiveSimulator,
    BlueprintEditor,
}

#[derive(PartialEq, Clone, Copy)]
enum PreviewState {
    IdleWithGlasses,
    ScaredNoGlasses,
    FlashedDefeat,
    JumpscareLoop,
}

struct FrickEditorApp {
    project: AnimatronicProject,
    mod_manager: ModManager,
    active_tab: ActiveTab,
    preview_state: PreviewState,
    grid_subview: GridSubView,
    salvage_tab_mode: SalvageTabMode,
    sim_time: f32,
    status_message: String,
    status_is_error: bool,
    tex_cache: HashMap<PathBuf, TextureHandle>,
    _audio_stream: Option<OutputStream>,
    audio_sink: Option<Sink>,
    // Motor Híbrido de Nodos y Simulación
    node_state: node_editor::FrickEditorState,
    node_custom_state: node_editor::NodeGraphCustomState,
    selected_code_file: usize,
    sim_blind_or_door_closed: bool,
    sim_flashlight_on: bool,
    // Visor y Debugger de Locaciones 3D / Salas (MinigameMaze 1 a 5)
    maze_rooms: HashMap<String, MazeRoomData>,
    selected_salvage_location: SalvageLocationIndex,
    salvage_flashlight: bool,
    salvage_cam_distance: f32,
    salvage_cam_yaw: f32,
    salvage_sim_paused: bool,
    // Debugger y Filtros de Entidades en Plano 2D
    dbg_filter_walls: bool,
    dbg_filter_props: bool,
    dbg_filter_interactive: bool,
    dbg_filter_spawns: bool,
    dbg_selected_instance_id: Option<i64>,
}

impl FrickEditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(18, 19, 23);
        visuals.panel_fill = Color32::from_rgb(22, 24, 29);
        visuals.faint_bg_color = Color32::from_rgb(28, 31, 38);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 31, 38);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(36, 40, 49);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 56, 68);
        visuals.widgets.active.bg_fill = Color32::from_rgb(215, 130, 35);
        visuals.selection.bg_fill = Color32::from_rgb(215, 130, 35);
        cc.egui_ctx.set_visuals(visuals);

        let (stream, sink) = match OutputStream::try_default() {
            Ok((s, handle)) => {
                let sk = Sink::try_new(&handle).ok();
                (Some(s), sk)
            }
            Err(_) => (None, None),
        };

        let default_proj = AnimatronicProject::default();
        let mods_dir = default_proj.workspace_dir.join("mods");
        let mut mod_manager = ModManager::new(mods_dir);
        mod_manager.add_or_update_mod(default_proj.clone());
        let _ = mod_manager.scan_mods(&default_proj.workspace_dir);

        let (_graph, node_state) = node_editor::create_default_murray_graph();

        Self {
            project: default_proj,
            mod_manager,
            active_tab: ActiveTab::ModManager,
            preview_state: PreviewState::IdleWithGlasses,
            grid_subview: GridSubView::CustomNightGrid,
            salvage_tab_mode: SalvageTabMode::LiveSimulator,
            sim_time: 0.0,
            status_message: "FrickEditor Studio listo. Gestor de Mods y Motor Híbrido cargados.".to_string(),
            status_is_error: false,
            tex_cache: HashMap::new(),
            _audio_stream: stream,
            audio_sink: sink,
            node_state,
            node_custom_state: node_editor::NodeGraphCustomState::default(),
            selected_code_file: 0,
            sim_blind_or_door_closed: false,
            sim_flashlight_on: false,
            maze_rooms: models::load_default_maze_rooms(),
            selected_salvage_location: SalvageLocationIndex::Location1Fnaf1,
            salvage_flashlight: true,
            salvage_cam_distance: 0.0,
            salvage_cam_yaw: 0.0,
            salvage_sim_paused: false,
            dbg_filter_walls: true,
            dbg_filter_props: true,
            dbg_filter_interactive: true,
            dbg_filter_spawns: true,
            dbg_selected_instance_id: None,
        }
    }

    fn handle_one_click_play(&mut self) {
        // 1. Sincronizar el proyecto actual en el gestor
        self.mod_manager.add_or_update_mod(self.project.clone());

        // 2. Verificar y auto-resolver conflictos de slots si existen
        let conflicts = self.mod_manager.check_conflicts();
        if !conflicts.is_empty() {
            self.mod_manager.auto_resolve_slots();
            if let Some(p) = self.mod_manager.installed_mods.iter().find(|m| m.internal_id == self.project.internal_id) {
                self.project.custom_night.slot_index = p.custom_night.slot_index;
            }
        }

        // 3. Obtener mods activos
        let mut active_mods = self.mod_manager.get_active_mods();
        if active_mods.is_empty() {
            self.mod_manager.set_active(&self.project.internal_id, true);
            active_mods = vec![self.project.clone()];
        }

        self.status_message = format!("Inyectando silenciosamente {} mod(s) y lanzando juego...", active_mods.len());
        self.status_is_error = false;

        // 4. Inyección silenciosa + Iniciar Frickbear's 3
        match GameLauncher::build_inject_and_play(
            &active_mods,
            &self.project.utmt_scripts_dir,
            &self.project.game_dir,
        ) {
            Ok(msg) => {
                self.status_is_error = false;
                self.status_message = msg;
            }
            Err(err) => {
                self.status_is_error = true;
                self.status_message = err;
            }
        }
    }

    fn get_or_load_tex(&mut self, ctx: &egui::Context, path: &Path, max_dim: u32) -> Option<TextureHandle> {
        if let Some(handle) = self.tex_cache.get(path) {
            return Some(handle.clone());
        }
        if !path.exists() {
            return None;
        }
        if let Ok(img) = image::open(path) {
            let thumb = img.thumbnail(max_dim, max_dim).to_rgba8();
            let size = [thumb.width() as usize, thumb.height() as usize];
            let color_img = ColorImage::from_rgba_unmultiplied(size, thumb.as_flat_samples().as_slice());
            let handle = ctx.load_texture(path.display().to_string(), color_img, TextureOptions::NEAREST);
            self.tex_cache.insert(path.to_path_buf(), handle.clone());
            return Some(handle);
        }
        None
    }

    fn play_audio(&mut self, path: &Path) {
        if let Some(sink) = &self.audio_sink {
            sink.stop();
            if let Ok(file) = File::open(path) {
                if let Ok(source) = Decoder::new(BufReader::new(file)) {
                    sink.append(source);
                    sink.play();
                    self.status_is_error = false;
                    self.status_message = format!("Reproduciendo: {}", path.file_name().unwrap_or_default().to_string_lossy());
                }
            }
        }
    }

    fn stop_audio(&mut self) {
        if let Some(sink) = &self.audio_sink {
            sink.stop();
        }
    }
}

impl eframe::App for FrickEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt);
        if !(self.active_tab == ActiveTab::Environment3DBuilder && self.salvage_sim_paused) {
            self.sim_time += dt;
        }
        ctx.request_repaint();

        // ==========================================================
        // BARRA SUPERIOR
        // ==========================================================
        egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}  FRICKEDITOR STUDIO", ic::CUBE))
                        .size(19.0)
                        .strong()
                        .color(Color32::from_rgb(245, 165, 60)),
                );

                ui.add_space(10.0);
                let play_header_btn = egui::Button::new(
                    RichText::new(format!("{}  PLAY (Jugar)", ic::PLAY))
                        .size(13.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(220, 95, 20))
                .min_size(Vec2::new(140.0, 28.0));

                if ui.add(play_header_btn).clicked() {
                    self.handle_one_click_play();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("{}  Guardar JSON", ic::FLOPPY_DISK)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Proyecto FrickEditor", &["json"])
                            .set_file_name("murray_project.json")
                            .save_file()
                        {
                            if let Ok(json) = serde_json::to_string_pretty(&self.project) {
                                let _ = std::fs::write(path, json);
                                self.status_is_error = false;
                                self.status_message = "Proyecto guardado correctamente.".to_string();
                            }
                        }
                    }

                    if ui.button(format!("{}  Cargar JSON", ic::FOLDER_OPEN)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Proyecto FrickEditor", &["json"])
                            .pick_file()
                        {
                            if let Ok(data) = std::fs::read_to_string(path) {
                                if let Ok(loaded) = serde_json::from_str::<AnimatronicProject>(&data) {
                                    self.project = loaded;
                                    self.tex_cache.clear();
                                    self.status_is_error = false;
                                    self.status_message = "Proyecto cargado en memoria.".to_string();
                                }
                            }
                        }
                    }

                    if ui.button(format!("{}  Exportar .fbmod", ic::UPLOAD_SIMPLE)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Frickbear Mod Package", &["fbmod"])
                            .set_file_name(format!("{}.fbmod", self.project.internal_id))
                            .save_file()
                        {
                            match IdeBuilder::export_fbmod_package(&self.project, &path) {
                                Ok(msg) => {
                                    self.status_is_error = false;
                                    self.status_message = msg;
                                }
                                Err(err) => {
                                    self.status_is_error = true;
                                    self.status_message = err;
                                }
                            }
                        }
                    }

                    if ui.button(format!("{}  Importar .fbmod", ic::DOWNLOAD_SIMPLE)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Frickbear Mod Package", &["fbmod", "zip"])
                            .pick_file()
                        {
                            let ws = self.project.workspace_dir.clone();
                            match IdeBuilder::import_fbmod_package(&path, &ws) {
                                Ok(loaded_proj) => {
                                    self.project = loaded_proj.clone();
                                    self.mod_manager.add_or_update_mod(loaded_proj);
                                    self.tex_cache.clear();
                                    self.status_is_error = false;
                                    self.status_message = "Paquete .fbmod importado y descomprimido con éxito.".to_string();
                                }
                                Err(err) => {
                                    self.status_is_error = true;
                                    self.status_message = err;
                                }
                            }
                        }
                    }

                    let (ready, total) = self.project.readiness_score();
                    let badge_col = if ready == total {
                        Color32::from_rgb(65, 195, 105)
                    } else {
                        Color32::from_rgb(235, 175, 55)
                    };
                    ui.label(
                        RichText::new(format!("{}  Assets verificados: {}/{}", ic::CHECK_CIRCLE, ready, total))
                            .color(badge_col)
                            .strong(),
                    );
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // Las 7 Pestañas del Estudio
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::ModManager,
                    RichText::new(format!("{}  0. Gestor de Mods & PLAY", ic::GAME_CONTROLLER)).size(13.0),
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Gallery,
                    RichText::new(format!("{}  1. Mod Gallery", ic::IDENTIFICATION_CARD)).size(13.0),
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::SpritesAndGuide,
                    RichText::new(format!("{}  2. Sprites & Audios", ic::IMAGE)).size(13.0),
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::LiveAnimationPreview,
                    RichText::new(format!("{}  3. Animación 60 FPS", ic::FILM_STRIP)).size(13.0),
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::GridAndMechanics,
                    RichText::new(format!("{}  4. Cuadrícula & IA", ic::GRID_FOUR)).size(13.0),
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::CameraMapRouter,
                    RichText::new(format!("{}  5. Ruteo de Cámaras", ic::MAP_TRIFOLD)).size(13.0),
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Environment3DBuilder,
                    RichText::new(format!("{}  6. Visor & Debugger 3D", ic::EYE))
                        .size(13.0)
                        .color(Color32::from_rgb(110, 230, 255))
                        .strong(),
                );
            });
            ui.add_space(4.0);
        });

        // ==========================================================
        // BARRA INFERIOR
        // ==========================================================
        egui::TopBottomPanel::bottom("footer_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let play_footer_btn = egui::Button::new(
                    RichText::new(format!("{}  PLAY (Inyectar y Jugar)", ic::PLAY))
                        .size(14.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(220, 95, 20))
                .min_size(Vec2::new(210.0, 38.0));

                if ui.add(play_footer_btn).clicked() {
                    self.handle_one_click_play();
                }

                ui.add_space(6.0);
                if ui.button(RichText::new(format!("{} Iniciar Frickbears 3.exe", ic::GAME_CONTROLLER)).size(13.0)).clicked() {
                    if let Some(game_exe) = GameLauncher::find_game_executable(&self.project.game_dir) {
                        match GameLauncher::launch_game(&game_exe) {
                            Ok(_) => {
                                self.status_message = format!("Juego iniciado: {}", game_exe.file_name().unwrap_or_default().to_string_lossy());
                                self.status_is_error = false;
                            }
                            Err(e) => {
                                self.status_message = format!("Error iniciando juego: {}", e);
                                self.status_is_error = true;
                            }
                        }
                    } else {
                        self.status_message = "No se encontró el ejecutable del juego en game_dir.".to_string();
                        self.status_is_error = true;
                    }
                }

                ui.add_space(10.0);
                let btn = egui::Button::new(
                    RichText::new(format!("{}  Exportar Todo (Galería, Sprites 640x640 y GML)", ic::ROCKET_LAUNCH))
                        .size(14.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(180, 100, 25))
                .min_size(Vec2::new(320.0, 38.0));

                if ui.add(btn).clicked() {
                    match IdeBuilder::build_full_ide_project(&self.project) {
                        Ok(msg) => {
                            self.status_is_error = false;
                            self.status_message = msg;
                        }
                        Err(err) => {
                            self.status_is_error = true;
                            self.status_message = err;
                        }
                    }
                }

                ui.add_space(10.0);
                if ui.button(format!("{}  Limpiar del juego", ic::BROOM)).clicked() {
                    match IdeBuilder::clean_from_game(&self.project) {
                        Ok(msg) => {
                            self.status_is_error = false;
                            self.status_message = msg;
                        }
                        Err(err) => {
                            self.status_is_error = true;
                            self.status_message = err;
                        }
                    }
                }

                ui.add_space(10.0);
                let icon = if self.status_is_error { ic::WARNING_CIRCLE } else { ic::CHECK_CIRCLE };
                let col = if self.status_is_error {
                    Color32::from_rgb(240, 85, 85)
                } else {
                    Color32::from_gray(200)
                };
                ui.label(RichText::new(format!("{}  {}", icon, self.status_message)).color(col));
            });
            ui.add_space(8.0);
        });

        // ==========================================================
        // PANEL CENTRAL
        // ==========================================================
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(27, 30, 37))
                .rounding(Rounding::same(6.0))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.columns(3, |cols| {
                        cols[0].horizontal(|ui| {
                            ui.label(RichText::new(format!("{} ID:", ic::TAG)).strong());
                            ui.text_edit_singleline(&mut self.project.internal_id);
                        });
                        Self::compact_folder_picker(&mut cols[1], "Juego:", &mut self.project.game_dir);
                        Self::compact_folder_picker(&mut cols[2], "Workspace:", &mut self.project.workspace_dir);
                    });
                });

            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                ActiveTab::ModManager => self.ui_mod_manager_tab(ctx, ui),
                ActiveTab::Gallery => self.ui_gallery_tab(ctx, ui),
                ActiveTab::SpritesAndGuide => self.ui_sprites_and_audio_tab(ctx, ui),
                ActiveTab::LiveAnimationPreview => self.ui_live_animation_tab(ctx, ui),
                ActiveTab::GridAndMechanics => self.ui_grid_and_mechanics_tab(ctx, ui),
                ActiveTab::CameraMapRouter => self.ui_camera_map_router_tab(ctx, ui),
                ActiveTab::Environment3DBuilder => self.ui_environment_3d_builder_tab(ctx, ui),
            });
        });
    }
}

impl FrickEditorApp {
    // --------------------------------------------------------------
    // PESTAÑA 0: GESTOR DE MODS Y LANZADOR DIRECTO (ONE-CLICK PLAY)
    // --------------------------------------------------------------
    fn ui_mod_manager_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. BANNER DE CONFLICTOS
        let conflicts = self.mod_manager.check_conflicts();
        if !conflicts.is_empty() {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(45, 18, 20))
                .stroke(Stroke::new(1.5_f32, Color32::from_rgb(230, 70, 70)))
                .rounding(Rounding::same(8.0))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(ic::WARNING_CIRCLE).size(26.0).color(Color32::from_rgb(245, 80, 80)));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("¡Se detectaron conflictos en los slots de Custom Night!")
                                    .strong()
                                    .color(Color32::WHITE)
                                    .size(14.5),
                            );
                            for c in &conflicts {
                                ui.label(RichText::new(format!("• {}", c)).color(Color32::from_rgb(255, 190, 190)).size(12.0));
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let fix_btn = egui::Button::new(
                                RichText::new(format!("{} Auto-resolver conflictos", ic::MAGIC_WAND))
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(190, 50, 45));
                            if ui.add(fix_btn).clicked() {
                                self.mod_manager.auto_resolve_slots();
                                self.status_is_error = false;
                                self.status_message = "Slots reasignados automáticamente a casillas libres (0..31).".to_string();
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }

        // 2. HERO DASHBOARD: PLAY (ONE-CLICK LAUNCHER)
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(25, 30, 40))
            .stroke(Stroke::new(1.2_f32, Color32::from_rgb(50, 75, 110)))
            .rounding(Rounding::same(8.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{}  PANEL DE JUEGO & INYECCIÓN SILENCIOSA", ic::ROCKET_LAUNCH))
                                .size(16.5)
                                .strong()
                                .color(Color32::from_rgb(245, 165, 60)),
                        );
                        let active_count = self.mod_manager.get_active_mods().len();
                        let total_count = self.mod_manager.installed_mods.len();
                        ui.label(
                            RichText::new(format!(
                                "Selecciona qué animatrónicos estarán activos en tu partida. Al presionar PLAY, FrickEditor unifica el código, ejecuta UndertaleModTool silenciosamente y abre Five Nights at Frickbear's 3.\nAnimatrónicos seleccionados: {} de {}",
                                active_count, total_count
                            ))
                            .size(12.0)
                            .color(Color32::from_gray(175)),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let play_btn = egui::Button::new(
                            RichText::new(format!("{}  PLAY (One-Click Play)", ic::PLAY))
                                .size(16.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(225, 95, 20))
                        .min_size(Vec2::new(230.0, 46.0));

                        if ui.add(play_btn).clicked() {
                            self.handle_one_click_play();
                        }
                    });
                });
            });

        ui.add_space(10.0);

        // 3. BARRA DE HERRAMIENTAS
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}  Animatrónicos Instalados:", ic::STACK)).strong().size(15.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{}  Escanear Carpeta", ic::ARROWS_CLOCKWISE)).clicked() {
                    let ws = self.project.workspace_dir.clone();
                    let n = self.mod_manager.scan_mods(&ws);
                    self.status_is_error = false;
                    self.status_message = format!("Escaneo completado: {} mods cargados.", n);
                }

                if ui.button(format!("{}  Importar .fbmod", ic::PLUS_CIRCLE)).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Paquete Frickbear", &["fbmod", "zip"])
                        .pick_file()
                    {
                        let ws = self.project.workspace_dir.clone();
                        match self.mod_manager.import_fbmod(&path, &ws) {
                            Ok(p) => {
                                self.status_is_error = false;
                                self.status_message = format!("Mod '{}' importado con éxito.", p.internal_id);
                            }
                            Err(e) => {
                                self.status_is_error = true;
                                self.status_message = e;
                            }
                        }
                    }
                }

                if ui.button(format!("{}  Sincronizar Actual", ic::DOWNLOAD_SIMPLE)).clicked() {
                    self.mod_manager.add_or_update_mod(self.project.clone());
                    self.status_is_error = false;
                    self.status_message = format!("Proyecto '{}' sincronizado al gestor.", self.project.internal_id);
                }

                if ui.button(format!("{}  Auto-resolver Slots", ic::MAGIC_WAND)).clicked() {
                    self.mod_manager.auto_resolve_slots();
                    self.status_is_error = false;
                    self.status_message = "Slots reorganizados automáticamente sin colisiones.".to_string();
                }

                if ui.button("Desactivar todos").clicked() {
                    let ids: Vec<String> = self.mod_manager.installed_mods.iter().map(|m| m.internal_id.clone()).collect();
                    for id in ids {
                        self.mod_manager.set_active(&id, false);
                    }
                }

                if ui.button("Activar todos").clicked() {
                    let ids: Vec<String> = self.mod_manager.installed_mods.iter().map(|m| m.internal_id.clone()).collect();
                    for id in ids {
                        self.mod_manager.set_active(&id, true);
                    }
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // 4. LISTA DE TARJETAS DE ANIMATRÓNICOS
        let mut to_load = None;
        let mut to_export = None;
        let mut to_remove = None;
        let mut to_toggle = Vec::new();
        let mut slot_changes = Vec::new();
        let num_mods = self.mod_manager.installed_mods.len();

        for i in 0..num_mods {
            let m = &self.mod_manager.installed_mods[i];
            let id = m.internal_id.clone();
            let full_name = m.gallery_info.full_name.clone();
            let arch_name = m.behavior.archetype.display_name();
            let debut = m.gallery_info.debut.clone();
            let va = m.gallery_info.va_credit.clone();
            let mut current_slot = m.custom_night.slot_index;
            let is_active = *self.mod_manager.active_mods.get(&id).unwrap_or(&false);

            let thumb_path = m.icon_48_path.clone().or_else(|| m.portrait_500_path.clone());
            let tex = thumb_path.and_then(|p| self.get_or_load_tex(ctx, &p, 48));

            let border_col = if is_active {
                Color32::from_rgb(60, 180, 95)
            } else {
                Color32::from_rgb(60, 65, 75)
            };

            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(24, 27, 34))
                .stroke(Stroke::new(1.0_f32, border_col))
                .rounding(Rounding::same(6.0))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Checkbox de activación
                        let mut active_flag = is_active;
                        if ui.checkbox(&mut active_flag, "").changed() {
                            to_toggle.push((id.clone(), active_flag));
                        }

                        // Miniatura del personaje
                        egui::Frame::none()
                            .fill(Color32::from_rgb(16, 18, 22))
                            .rounding(Rounding::same(4.0))
                            .show(ui, |ui| {
                                if let Some(t) = tex {
                                    ui.add(egui::Image::new(&t).fit_to_exact_size(Vec2::new(44.0, 44.0)));
                                } else {
                                    ui.allocate_ui(Vec2::new(44.0, 44.0), |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(RichText::new(ic::IMAGE).size(18.0).color(Color32::DARK_GRAY));
                                        });
                                    });
                                }
                            });

                        ui.add_space(8.0);

                        // Información principal
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&full_name)
                                        .strong()
                                        .color(Color32::WHITE)
                                        .size(14.5),
                                );
                                ui.label(
                                    RichText::new(format!("({})", id))
                                        .color(Color32::from_gray(160))
                                        .size(12.5),
                                );
                                if is_active {
                                    ui.label(
                                        RichText::new(format!("{} Activo", ic::CHECK_CIRCLE))
                                            .color(Color32::from_rgb(80, 210, 120))
                                            .size(11.5),
                                    );
                                }
                            });

                            ui.label(
                                RichText::new(format!(
                                    "Arquetipo: {} | Debut: {} | VA: {}",
                                    arch_name, debut, va
                                ))
                                .size(11.0)
                                .color(Color32::from_gray(150)),
                            );
                        });

                        // Configuración del Slot de Custom Night
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{} Slot:", ic::GRID_FOUR)).strong());
                            if ui.button("-").clicked() && current_slot > 0 {
                                current_slot -= 1;
                                slot_changes.push((i, current_slot));
                            }
                            ui.label(
                                RichText::new(format!("#{}", current_slot + 1))
                                    .strong()
                                    .color(Color32::from_rgb(240, 160, 50)),
                            );
                            if ui.button("+").clicked() && current_slot < 31 {
                                current_slot += 1;
                                slot_changes.push((i, current_slot));
                            }
                        });

                        // Botones de acción derecha
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(format!("{} Eliminar", ic::TRASH)).clicked() {
                                to_remove = Some(id.clone());
                            }

                            if ui.button(format!("{} Exportar .fbmod", ic::UPLOAD_SIMPLE)).clicked() {
                                to_export = Some(self.mod_manager.installed_mods[i].clone());
                            }

                            let edit_btn = egui::Button::new(
                                RichText::new(format!("{} Editar en Estudio", ic::NOTE_PENCIL))
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(45, 90, 140));

                            if ui.add(edit_btn).clicked() {
                                to_load = Some(self.mod_manager.installed_mods[i].clone());
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }

        for (idx, new_slot) in slot_changes {
            if idx < self.mod_manager.installed_mods.len() {
                self.mod_manager.installed_mods[idx].custom_night.slot_index = new_slot;
            }
        }

        for (id, val) in to_toggle {
            self.mod_manager.set_active(&id, val);
        }

        // Manejar acciones diferidas
        if let Some(loaded) = to_load {
            self.project = loaded;
            self.tex_cache.clear();
            self.active_tab = ActiveTab::Gallery;
            self.status_is_error = false;
            self.status_message = format!("Animatrónico '{}' cargado en las pestañas de edición.", self.project.internal_id);
        }

        if let Some(exp_proj) = to_export {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Frickbear Mod Package", &["fbmod"])
                .set_file_name(format!("{}.fbmod", exp_proj.internal_id))
                .save_file()
            {
                match IdeBuilder::export_fbmod_package(&exp_proj, &path) {
                    Ok(msg) => {
                        self.status_is_error = false;
                        self.status_message = msg;
                    }
                    Err(err) => {
                        self.status_is_error = true;
                        self.status_message = err;
                    }
                }
            }
        }

        if let Some(rem_id) = to_remove {
            self.mod_manager.remove_mod(&rem_id);
            self.status_is_error = false;
            self.status_message = format!("Mod '{}' retirado de la lista.", rem_id);
        }
    }

    // --------------------------------------------------------------
    // PESTAÑA 1: GALERÍA Y PREVISUALIZACIÓN IN-GAME
    // --------------------------------------------------------------
    fn ui_gallery_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            cols[0].heading(format!("{}  Configuración de Mod Gallery", ic::SLIDERS_HORIZONTAL));
            cols[0].add_space(6.0);

            egui::Grid::new("meta_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(&mut cols[0], |ui| {
                    ui.label("Nombre (FULL_NAME):");
                    ui.text_edit_singleline(&mut self.project.gallery_info.full_name);
                    ui.end_row();

                    ui.label("Año Debut (DEBUT):");
                    ui.text_edit_singleline(&mut self.project.gallery_info.debut);
                    ui.end_row();

                    ui.label("Actor de Voz (VACredit):");
                    ui.text_edit_singleline(&mut self.project.gallery_info.va_credit);
                    ui.end_row();
                });

            cols[0].add_space(8.0);
            cols[0].label(RichText::new("Descripción Lore (DESCRIPTION):").strong());
            cols[0].add(
                egui::TextEdit::multiline(&mut self.project.gallery_info.description)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY),
            );

            cols[0].add_space(8.0);
            cols[0].label(RichText::new("Notas de Desarrollo (DEV_NOTES):").strong());
            cols[0].add(
                egui::TextEdit::multiline(&mut self.project.gallery_info.dev_notes)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );

            cols[0].add_space(10.0);
            cols[0].separator();
            cols[0].heading(format!("{}  Imágenes de Galería y Casilla", ic::IMAGE));
            cols[0].add_space(6.0);

            let p_path = self.project.portrait_500_path.clone();
            let p_tex = p_path.as_deref().and_then(|p| self.get_or_load_tex(ctx, p, 160));
            Self::asset_card_row(
                &mut cols[0],
                "Portrait de Galería (portrait.png)",
                "RECOMENDACIÓN: 500x500 px PNG transparente. Se usa en el panel central de Extras.",
                &mut self.project.portrait_500_path,
                p_tex.clone(),
            );

            cols[0].add_space(6.0);
            let i_path = self.project.icon_48_path.clone();
            let i_tex = i_path.as_deref().and_then(|p| self.get_or_load_tex(ctx, p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "Icono de Casilla (icon.png & sTile_0)",
                "RECOMENDACIÓN: 48x48 px exactos. Se usa en el selector de Galería y en Custom Night.",
                &mut self.project.icon_48_path,
                i_tex.clone(),
            );

            cols[0].checkbox(
                &mut self.project.auto_generate_gray_tile,
                "Auto-generar Frame 1 en escala de grises (sTile_1) para AI=0 y Death Coin",
            );

            cols[0].add_space(8.0);
            if cols[0]
                .button(
                    RichText::new(format!("{}  Exportar Solo Ficha a Mod Gallery", ic::PACKAGE))
                        .strong()
                        .color(Color32::from_rgb(110, 230, 255)),
                )
                .clicked()
            {
                match IdeBuilder::export_standalone_gallery_addon(&self.project) {
                    Ok(msg) => {
                        self.status_is_error = false;
                        self.status_message = msg;
                    }
                    Err(err) => {
                        self.status_is_error = true;
                        self.status_message = err;
                    }
                }
            }
            cols[0].label(
                RichText::new("Crea/actualiza addons/<ID>/extras_info.txt, portrait.png e icon.png de forma independiente sin tocar data.win.")
                    .size(11.0)
                    .color(Color32::from_gray(140)),
            );

            // Columna Derecha: Vista previa en vivo
            cols[1].heading(format!("{}  Previsualización de Ficha en Mod Gallery", ic::EYE));
            cols[1].add_space(6.0);

            egui::Frame::group(cols[1].style())
                .fill(Color32::from_rgb(12, 12, 16))
                .stroke(Stroke::new(1.5_f32, Color32::from_rgb(190, 190, 190)))
                .inner_margin(14.0)
                .show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(192, 192, 192))
                            .stroke(Stroke::new(1.0_f32, Color32::WHITE))
                            .inner_margin(2.0)
                            .show(ui, |ui| {
                                if let Some(tex) = &i_tex {
                                    ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(48.0, 48.0)));
                                } else {
                                    ui.allocate_space(Vec2::new(48.0, 48.0));
                                }
                            });

                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&self.project.gallery_info.full_name)
                                    .size(18.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Debut: {}  |  VA Credit: {}",
                                    self.project.gallery_info.debut, self.project.gallery_info.va_credit
                                ))
                                .color(Color32::from_rgb(235, 165, 65)),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    if let Some(p_handle) = &p_tex {
                        ui.vertical_centered(|ui| {
                            ui.add(egui::Image::new(p_handle).fit_to_exact_size(Vec2::new(140.0, 140.0)));
                        });
                        ui.add_space(6.0);
                    }

                    ui.label(RichText::new("DESCRIPTION:").strong().color(Color32::from_rgb(130, 180, 250)));
                    ui.label(RichText::new(&self.project.gallery_info.description).color(Color32::from_gray(220)));
                    ui.add_space(8.0);
                    ui.label(RichText::new("DEV NOTES:").strong().color(Color32::from_rgb(130, 180, 250)));
                    ui.label(RichText::new(&self.project.gallery_info.dev_notes).color(Color32::from_gray(190)));
                });
        });
    }

    // --------------------------------------------------------------
    // PESTAÑA 2: SPRITES CON RECOMENDACIONES DE USO Y BANCO DE AUDIO
    // --------------------------------------------------------------
    fn ui_sprites_and_audio_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let verified = self.project.count_verified_assets();
            let badge_color = if verified == 15 { Color32::from_rgb(80, 220, 120) } else { Color32::from_rgb(255, 180, 40) };
            ui.label(RichText::new(format!("{} Checklist de Assets: {}/15 listos para montar", ic::CHECK_CIRCLE, verified)).strong().color(badge_color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new(format!("{}  Autocompletar Plantilla Completa (Sin Importar Nada)", ic::MAGIC_WAND)).strong().color(Color32::from_rgb(100, 240, 160))).clicked() {
                    match IdeBuilder::generate_default_placeholders(&mut self.project) {
                        Ok(msg) => {
                            self.status_is_error = false;
                            self.status_message = msg;
                            self.tex_cache.clear();
                        }
                        Err(err) => {
                            self.status_is_error = true;
                            self.status_message = err;
                        }
                    }
                }
            });
        });
        ui.add_space(8.0);

        ui.columns(2, |cols| {
            cols[0].heading(format!("{}  Sprites de Oficina (Con Guía de Uso en Animación)", ic::STACK));
            cols[0].add_space(6.0);

            let b_tex = self.project.body_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "1. Torso / Cuerpo (s<ID>_body)",
                "USO: Estado 1 (Ventana). Se dibuja en DrawDepth=48 con oscilación senoidal suave.",
                &mut self.project.body_sprite,
                b_tex,
            );
            cols[0].add_space(6.0);

            let h_tex = self.project.head_normal_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "2. Cabeza Confiada (s<ID>_head)",
                "USO: Estado 1 (Con gafas). Se dibuja en DrawDepth=50 mientras has_glasses == true.",
                &mut self.project.head_normal_sprite,
                h_tex,
            );
            cols[0].add_space(6.0);

            let hs_tex = self.project.head_scare_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "3. Cabeza Asustada (s<ID>_head_Scare)",
                "USO: Estado 1B (Sin gafas). Reemplaza a s_head al clicar las gafas; aplica temblor jitter.",
                &mut self.project.head_scare_sprite,
                hs_tex,
            );
            cols[0].add_space(6.0);

            let g_tex = self.project.accessory_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "4. Gafas Clicables (s<ID>_Glasses)",
                "USO: DrawDepth=52. Recibe el clic exclusivo del ratón y sale disparada con gravedad.",
                &mut self.project.accessory_sprite,
                g_tex,
            );
            cols[0].add_space(6.0);

            let fl_tex = self.project.flash_defeat_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "5. Derrotado por Flash (s<ID>_Flash)",
                "USO: Estado 2. Se muestra al alumbrarlo sin gafas mientras cae hacia abajo.",
                &mut self.project.flash_defeat_sprite,
                fl_tex,
            );
            cols[0].add_space(6.0);

            let alt_tex = self.project.head_gameover_alt.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "6. Cabeza Carmesí Game Over (s<ID>_head_Alt)",
                "USO: Sala GameOver. Se dibuja a escala 3.2x con tinte rojo detrás de las letras.",
                &mut self.project.head_gameover_alt,
                alt_tex,
            );

            // Columna Derecha: Jumpscare y Audios
            cols[1].heading(format!("{}  Capas del Jumpscare (Auto-Unión 640x640)", ic::LIGHTNING));
            cols[1].add_space(6.0);

            let j0_tex = self.project.jumpscare_frame_0.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[1],
                "Jumpscare Capa Base (sJumpscare_0)",
                "USO: Estado 4 (Draw_64). Torso naranja de 640x429 que se une con las cabezas.",
                &mut self.project.jumpscare_frame_0,
                j0_tex,
            );
            cols[1].add_space(6.0);

            let j1_tex = self.project.jumpscare_frame_1.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[1],
                "Jumpscare Cabeza 1 (sJumpscare_1)",
                "USO: Estado 4 (Draw_64). Alterna cada 3 frames con la Cabeza 2 durante el ataque.",
                &mut self.project.jumpscare_frame_1,
                j1_tex,
            );
            cols[1].add_space(6.0);

            let j2_tex = self.project.jumpscare_frame_2.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[1],
                "Jumpscare Cabeza 2 (sJumpscare_2)",
                "USO: Estado 4 (Draw_64). Mandíbula abierta; se congela en el frame 68 antes de morir.",
                &mut self.project.jumpscare_frame_2,
                j2_tex,
            );

            cols[1].add_space(10.0);
            cols[1].separator();
            cols[1].heading(format!("{}  Audios (.ogg) con Probador Integrado", ic::SPEAKER_HIGH));
            cols[1].add_space(6.0);

            let k1 = self.project.sound_knock_alert.clone();
            self.audio_card_row(&mut cols[1], "1. Golpe en Ventana (murray_knock.ogg)", "Suena al entrar al Estado 1", k1, 1);
            cols[1].add_space(6.0);

            let k2 = self.project.sound_voice_line.clone();
            self.audio_card_row(&mut cols[1], "2. Frase al Abrir Persiana (murray_amigable.ogg)", "Saludo al levantar la persiana izquierda", k2, 2);
            cols[1].add_space(6.0);

            let k3 = self.project.sound_defeat.clone();
            self.audio_card_row(&mut cols[1], "3. Caída Cómica de Alce (murray_knock2.ogg)", "Suena al flashearlo sin lentes", k3, 3);
            cols[1].add_space(6.0);

            let k4 = self.project.sound_jumpscare.clone();
            self.audio_card_row(&mut cols[1], "4. Jumpscare (murray_jumpscare.ogg)", "Suena durante el ataque Draw_64", k4, 4);
        });
    }

    // --------------------------------------------------------------
    // PESTAÑA 3: SIMULADOR DE ANIMACIÓN EN VIVO (60 FPS)
    // --------------------------------------------------------------
    fn ui_live_animation_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ctx.request_repaint(); // Mantener 60 FPS solo mientras estamos en la pestaña de simulación
        self.sim_time += ctx.input(|i| i.stable_dt);

        ui.horizontal(|ui| {
            ui.heading(format!("{}  Previsualizador de Estados y Animación en Tiempo Real", ic::FILM_STRIP));
            ui.add_space(16.0);
            ui.selectable_value(&mut self.preview_state, PreviewState::IdleWithGlasses, "Estado 1: En Ventana (Con Gafas)");
            ui.selectable_value(&mut self.preview_state, PreviewState::ScaredNoGlasses, "Estado 1B: Asustado (Gafas Volando)");
            ui.selectable_value(&mut self.preview_state, PreviewState::FlashedDefeat, "Estado 2: Derrotado por Flash");
            ui.selectable_value(&mut self.preview_state, PreviewState::JumpscareLoop, "Estado 4: Jumpscare (60 FPS)");
        });
        ui.add_space(8.0);

        let body_t = self.project.body_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let head_t = self.project.head_normal_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let scare_t = self.project.head_scare_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let glass_t = self.project.accessory_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 128));
        let flash_t = self.project.flash_defeat_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let j0_t = self.project.jumpscare_frame_0.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
        let j1_t = self.project.jumpscare_frame_1.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
        let j2_t = self.project.jumpscare_frame_2.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));

        // Lienzo de simulación de la ventana de la oficina
        let (rect, _resp) = ui.allocate_exact_size(Vec2::new(640.0, 380.0), Sense::hover());
        let painter = ui.painter_at(rect);

        // Marco de la ventana trasera de la oficina
        painter.rect_filled(rect, 6.0, Color32::from_rgb(10, 12, 16));
        painter.rect_stroke(rect, 6.0, Stroke::new(3.0_f32, Color32::from_rgb(90, 95, 110)));

        let center = rect.center();
        let t = self.sim_time;

        match self.preview_state {
            PreviewState::IdleWithGlasses => {
                let bob_body = (t * 3.0).sin() * 4.0;
                let bob_head = (t * 4.0).sin() * 5.0;

                if let Some(tex) = &body_t {
                    let r = Rect::from_center_size(Pos2::new(center.x, center.y + 75.0 + bob_body), Vec2::new(190.0, 140.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                if let Some(tex) = &head_t {
                    let r = Rect::from_center_size(Pos2::new(center.x, center.y - 10.0 + bob_head), Vec2::new(170.0, 145.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                if let Some(tex) = &glass_t {
                    let gx = center.x + (self.project.behavior.accessory_offset_x as f32 - 52.0);
                    let gy = center.y - 8.0 + bob_head + (self.project.behavior.accessory_offset_y as f32 - 48.0);
                    let r = Rect::from_center_size(Pos2::new(gx, gy), Vec2::new(76.0, 30.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    // Dibujar recuadro verde del Hitbox exclusivo de las gafas
                    painter.rect_stroke(r, 2.0, Stroke::new(1.5_f32, Color32::from_rgb(50, 220, 100)));
                }
            }
            PreviewState::ScaredNoGlasses => {
                let jitter_x = (t * 45.0).sin() * 3.5;
                let jitter_y = (t * 55.0).cos() * 3.5;

                if let Some(tex) = &body_t {
                    let r = Rect::from_center_size(Pos2::new(center.x, center.y + 75.0), Vec2::new(190.0, 140.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                if let Some(tex) = &scare_t {
                    let r = Rect::from_center_size(Pos2::new(center.x + jitter_x, center.y - 10.0 + jitter_y), Vec2::new(175.0, 150.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                // Simular parábola repetitiva de las gafas saliendo disparadas
                if let Some(tex) = &glass_t {
                    let loop_t = (t % 1.2) / 1.2;
                    let gx = center.x + (loop_t * 180.0);
                    let gy = (center.y - 15.0) - (loop_t * 90.0) + (loop_t * loop_t * 280.0);
                    let r = Rect::from_center_size(Pos2::new(gx, gy), Vec2::new(76.0, 30.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            }
            PreviewState::FlashedDefeat => {
                let fall_t = (t % 1.5) / 1.5;
                let shake_x = (t * 60.0).sin() * 5.0;
                let drop_y = if fall_t > 0.4 { (fall_t - 0.4) * 220.0 } else { 0.0 };

                if let Some(tex) = &flash_t {
                    let r = Rect::from_center_size(Pos2::new(center.x + shake_x, center.y + drop_y), Vec2::new(220.0, 220.0));
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            }
            PreviewState::JumpscareLoop => {
                let cycle = (t % 1.4) / 1.4;
                let zoom = 0.55 + (cycle.powf(0.45) * 0.70);
                let shake_x = (t * 70.0).sin() * 8.0 * cycle;
                let shake_y = (t * 85.0).cos() * 8.0 * cycle;
                let sz = Vec2::new(340.0 * zoom, 340.0 * zoom);
                let anchor_pos = Pos2::new(center.x + shake_x, center.y + (20.0 * zoom) + shake_y);

                if let Some(tex) = &j0_t {
                    let r = Rect::from_center_size(anchor_pos, sz);
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                let head_choice = if ((t * 15.0) as i32) % 2 == 0 { &j1_t } else { &j2_t };
                if let Some(tex) = head_choice {
                    let jaw_y = (t * 35.0).sin() * 3.5 * zoom;
                    let r = Rect::from_center_size(Pos2::new(anchor_pos.x, anchor_pos.y + jaw_y), sz);
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            }
        }
    }

    // --------------------------------------------------------------
    // PESTAÑA 4: MOTOR DE COMPORTAMIENTO HÍBRIDO (NODOS / GML) & CUADRÍCULA
    // --------------------------------------------------------------
    fn ui_grid_and_mechanics_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Encabezado con selector híbrido
        ui.horizontal(|ui| {
            ui.heading(format!("{} Motor de Comportamiento e IA", ic::CPU));
            ui.add_space(20.0);

            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(20, 24, 30))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(100, 150, 255)))
                .rounding(Rounding::same(6.0))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.project.advanced.use_node_editor,
                            true,
                            format!("{} Drag & Drop (Nodos Visuales)", ic::FLOW_ARROW),
                        );
                        ui.selectable_value(
                            &mut self.project.advanced.use_node_editor,
                            false,
                            format!("{} Código Manual (GML Puro)", ic::CODE),
                        );
                    });
                });
        });

        ui.add_space(8.0);

        // Previsualizador 60 FPS texturas
        let body_t = self.project.body_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let head_t = self.project.head_normal_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let scare_t = self.project.head_scare_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let glass_t = self.project.accessory_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 128));
        let flash_t = self.project.flash_defeat_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 256));
        let j0_t = self.project.jumpscare_frame_0.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
        let j1_t = self.project.jumpscare_frame_1.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
        let j2_t = self.project.jumpscare_frame_2.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));

        let t = self.sim_time;

        ui.columns(2, |cols| {
            // COLUMNA IZQUIERDA: Área de Programación (Nodos o GML)
            if self.project.advanced.use_node_editor {
                cols[0].horizontal(|ui| {
                    ui.label(RichText::new("Editor de Nodos Lógicos:").strong().color(Color32::from_rgb(100, 180, 255)));
                    ui.label(RichText::new("(Clic derecho para añadir nodos)").size(11.0).color(Color32::GRAY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{} Traducir a GML", ic::ARROWS_CLOCKWISE)).clicked() {
                            let gml = node_editor::generate_gml_from_graph(&self.node_state.graph, &self.project.internal_id);
                            let step_key = format!("gml_Object_o{}_BackWindow_Step_0.gml", self.project.internal_id);
                            self.project.behavior.custom_gml_overrides.insert(step_key, gml);
                            self.status_message = "Grafo de nodos sincronizado y traducido a GML con éxito.".to_string();
                            self.status_is_error = false;
                        }
                    });
                });

                cols[0].add_space(4.0);

                let (rect, _) = cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), 460.0), Sense::click_and_drag());
                let painter = cols[0].painter_at(rect);
                painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 20, 26));
                painter.rect_stroke(rect, 6.0, Stroke::new(1.0_f32, Color32::from_rgb(55, 60, 75)));

                cols[0].allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
                    node_editor::draw_node_graph_ui(ui, &mut self.node_state, &mut self.node_custom_state);
                });
            } else {
                cols[0].label(RichText::new("Editor GML Avanzado: Sobrescritura directa de los scripts.").strong().color(Color32::from_rgb(255, 170, 50)));
                cols[0].add_space(4.0);

                let gml_files = crate::builder::IdeBuilder::get_effective_gml_scripts(&self.project);
                let file_names: Vec<String> = gml_files.keys().cloned().collect();

                if self.selected_code_file >= file_names.len() {
                    self.selected_code_file = 0;
                }

                if let Some(selected_key) = file_names.get(self.selected_code_file) {
                    cols[0].horizontal(|ui| {
                        ui.label(RichText::new("Script Activo:").strong());
                        egui::ComboBox::from_label("")
                            .selected_text(selected_key)
                            .show_ui(ui, |ui| {
                                for (idx, name) in file_names.iter().enumerate() {
                                    ui.selectable_value(&mut self.selected_code_file, idx, name);
                                }
                            });

                        if ui.button(format!("{} Revertir a Plantilla", ic::ARROW_COUNTER_CLOCKWISE)).clicked() {
                            self.project.behavior.custom_gml_overrides.remove(selected_key);
                        }
                    });

                    cols[0].add_space(4.0);

                    let mut active_code = self.project.behavior.custom_gml_overrides
                        .get(selected_key)
                        .cloned()
                        .unwrap_or_else(|| gml_files.get(selected_key).cloned().unwrap_or_default());

                    egui::ScrollArea::vertical().id_salt("gml_editor_scroll").max_height(420.0).show(&mut cols[0], |ui| {
                        if ui.add(egui::TextEdit::multiline(&mut active_code).code_editor().desired_width(f32::INFINITY)).changed() {
                            self.project.behavior.custom_gml_overrides.insert(selected_key.clone(), active_code);
                        }
                    });
                }
            }

            // COLUMNA DERECHA: Simulador Interactivo a 60 FPS
            cols[1].heading(format!("{} Simulador Interactivo de Oficina", ic::MONITOR_PLAY));
            cols[1].add_space(4.0);

            let (rect, resp) = cols[1].allocate_exact_size(Vec2::new(cols[1].available_width(), 310.0), Sense::click());
            let painter = cols[1].painter_at(rect);
            painter.rect_filled(rect, 6.0, Color32::from_rgb(10, 12, 16));
            painter.rect_stroke(rect, 6.0, Stroke::new(2.0_f32, Color32::from_rgb(70, 75, 90)));

            let center = rect.center();

            // Renderizado de animatrónico
            match self.preview_state {
                PreviewState::IdleWithGlasses => {
                    let bob_body = (t * 3.0).sin() * 4.0;
                    let bob_head = (t * 4.0).sin() * 5.0;
                    if let Some(tex) = &body_t {
                        let r = Rect::from_center_size(Pos2::new(center.x, center.y + 65.0 + bob_body), Vec2::new(170.0, 125.0));
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    if let Some(tex) = &head_t {
                        let r = Rect::from_center_size(Pos2::new(center.x, center.y - 12.0 + bob_head), Vec2::new(150.0, 130.0));
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    if let Some(tex) = &glass_t {
                        let gx = center.x + (self.project.behavior.accessory_offset_x as f32 - 52.0);
                        let gy = center.y - 10.0 + bob_head + (self.project.behavior.accessory_offset_y as f32 - 48.0);
                        let r = Rect::from_center_size(Pos2::new(gx, gy), Vec2::new(68.0, 26.0));
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                        painter.rect_stroke(r, 2.0, Stroke::new(1.5_f32, Color32::from_rgb(50, 220, 100)));
                    }
                }
                PreviewState::ScaredNoGlasses => {
                    let jitter_x = (t * 45.0).sin() * 3.5;
                    let jitter_y = (t * 55.0).cos() * 3.5;
                    if let Some(tex) = &body_t {
                        let r = Rect::from_center_size(Pos2::new(center.x, center.y + 65.0), Vec2::new(170.0, 125.0));
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    if let Some(tex) = &scare_t {
                        let r = Rect::from_center_size(Pos2::new(center.x + jitter_x, center.y - 12.0 + jitter_y), Vec2::new(155.0, 135.0));
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                }
                PreviewState::FlashedDefeat => {
                    let fall_t = (t % 1.5) / 1.5;
                    let shake_x = (t * 60.0).sin() * 5.0;
                    let drop_y = if fall_t > 0.4 { (fall_t - 0.4) * 200.0 } else { 0.0 };
                    if let Some(tex) = &flash_t {
                        let r = Rect::from_center_size(Pos2::new(center.x + shake_x, center.y + drop_y), Vec2::new(190.0, 190.0));
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                }
                PreviewState::JumpscareLoop => {
                    let cycle = (t % 1.4) / 1.4;
                    let zoom = 0.55 + (cycle.powf(0.45) * 0.70);
                    let sz = Vec2::new(300.0 * zoom, 300.0 * zoom);
                    let anchor_pos = Pos2::new(center.x, center.y + (15.0 * zoom));
                    if let Some(tex) = &j0_t {
                        let r = Rect::from_center_size(anchor_pos, sz);
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    let head_choice = if ((t * 15.0) as i32) % 2 == 0 { &j1_t } else { &j2_t };
                    if let Some(tex) = head_choice {
                        let jaw_y = (t * 35.0).sin() * 3.5 * zoom;
                        let r = Rect::from_center_size(Pos2::new(anchor_pos.x, anchor_pos.y + jaw_y), sz);
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                }
            }

            // Si la persiana está cerrada en la simulación
            if self.sim_blind_or_door_closed {
                painter.rect_filled(rect, 6.0, Color32::from_rgb(32, 35, 44));
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("{} PERSIANA CERRADA\nAnimatrónico no puede atacar (Regla #3)", ic::LOCK_KEY),
                    egui::FontId::proportional(15.0),
                    Color32::from_rgb(220, 220, 240),
                );
            }

            // Efecto de linterna
            if self.sim_flashlight_on && !self.sim_blind_or_door_closed {
                painter.rect_filled(rect, 6.0, Color32::from_rgba_unmultiplied(255, 255, 170, 45));
            }

            if resp.clicked() && !self.sim_blind_or_door_closed {
                self.preview_state = PreviewState::ScaredNoGlasses;
            }

            // Controles interactivos del simulador
            cols[1].add_space(6.0);
            cols[1].horizontal(|ui| {
                ui.toggle_value(&mut self.sim_blind_or_door_closed, format!("{} Persiana", ic::LOCK_KEY));
                ui.toggle_value(&mut self.sim_flashlight_on, format!("{} Linterna (F)", ic::FLASHLIGHT));
                if ui.button(format!("{} Clic en Gafas", ic::CURSOR_CLICK)).clicked() {
                    self.preview_state = PreviewState::ScaredNoGlasses;
                }
            });

            cols[1].add_space(4.0);
            cols[1].horizontal(|ui| {
                if ui.button(format!("{} Flash Defeat", ic::SUN)).clicked() {
                    self.preview_state = PreviewState::FlashedDefeat;
                }
                if ui.button(format!("{} Jumpscare", ic::GHOST)).clicked() {
                    self.preview_state = PreviewState::JumpscareLoop;
                }
                if ui.button(format!("{} Normal", ic::ARROWS_CLOCKWISE)).clicked() {
                    self.preview_state = PreviewState::IdleWithGlasses;
                }
            });

            cols[1].add_space(6.0);
            egui::Frame::group(cols[1].style())
                .fill(Color32::from_rgb(24, 27, 34))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 80)))
                .rounding(Rounding::same(6.0))
                .inner_margin(6.0)
                .show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(ic::SHIELD_CHECK).size(18.0).color(Color32::from_rgb(80, 210, 130)));
                        ui.label(
                            RichText::new("Invariante Rule #3: Si la persiana está cerrada (!my_blind.Open), el animatrónico tiene prohibido atacar.")
                                .size(11.5)
                                .color(Color32::from_gray(190)),
                        );
                    });
                });
        });

        // SECCIÓN INFERIOR: Cuadrícula de Custom Night & Sliders
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        let icon_tex = self.project.icon_48_path.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));

        ui.columns(2, |cols| {
            // Columna Izquierda: Selector de Vistas (Cuadrícula, Visor 4LD, Menú de Pausa)
            cols[0].horizontal(|ui| {
                ui.selectable_value(&mut self.grid_subview, GridSubView::CustomNightGrid, format!("{} Cuadrícula Mods", ic::GRID_FOUR));
                ui.selectable_value(&mut self.grid_subview, GridSubView::FourLayersDownSelector, format!("{} Visor 4LD", ic::GAME_CONTROLLER));
                ui.selectable_value(&mut self.grid_subview, GridSubView::PauseMenuPreview, format!("{} Menú de Pausa", ic::PAUSE));
            });
            cols[0].add_space(4.0);

            // Banner de Rol Invariante
            egui::Frame::group(cols[0].style())
                .fill(Color32::from_rgb(32, 22, 22))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(220, 80, 80)))
                .inner_margin(6.0)
                .show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(ic::SHIELD_WARNING).size(16.0).color(Color32::from_rgb(255, 110, 110)));
                        ui.label(
                            RichText::new("ROL: ANIMATRÓNICO RIVAL / ENEMIGO (Custom Night, Endless, Salvage) — NUNCA GUARDIA")
                                .size(11.0)
                                .color(Color32::from_rgb(255, 190, 190))
                                .strong(),
                        );
                    });
                });
            cols[0].add_space(6.0);

            match self.grid_subview {
                GridSubView::CustomNightGrid => {
                    cols[0].heading(format!("{}  Posición del Tile en Custom Night", ic::GRID_FOUR));
                    cols[0].label("Haz clic en cualquier casilla para elegir en qué orden aparece en la Página 2 (8x7 = 56 Slots):");
                    cols[0].add_space(4.0);

                    egui::Frame::group(cols[0].style())
                        .fill(Color32::BLACK)
                        .stroke(Stroke::new(1.5_f32, Color32::from_gray(160)))
                        .inner_margin(8.0)
                        .show(&mut cols[0], |ui| {
                            egui::Grid::new("custom_night_page2_grid")
                                .spacing([6.0, 6.0])
                                .show(ui, |ui| {
                                    for slot in 0..32 {
                                        let is_selected = self.project.custom_night.slot_index == slot;
                                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(36.0, 42.0), Sense::click());
                                        if resp.clicked() {
                                            self.project.custom_night.slot_index = slot;
                                        }

                                        let p = ui.painter_at(rect);
                                        let bg = if is_selected {
                                            Color32::from_rgb(220, 140, 30)
                                        } else if resp.hovered() {
                                            Color32::from_rgb(65, 70, 85)
                                        } else {
                                            Color32::from_rgb(35, 38, 45)
                                        };
                                        p.rect_filled(rect, 3.0, bg);

                                        if is_selected {
                                            if let Some(tex) = &icon_tex {
                                                let img_r = Rect::from_min_size(rect.min + Vec2::new(2.0, 2.0), Vec2::new(32.0, 32.0));
                                                p.image(tex.id(), img_r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                                            }
                                            p.rect_stroke(rect, 3.0, Stroke::new(2.0_f32, Color32::YELLOW));
                                        } else {
                                            p.text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                format!("#{}", slot + 1),
                                                egui::FontId::proportional(11.0),
                                                Color32::from_gray(130),
                                            );
                                        }

                                        if (slot + 1) % 8 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    let (tx, ty) = self.project.custom_night.slot_to_xy();
                    cols[0].add_space(4.0);
                    cols[0].label(
                        RichText::new(format!(
                            "{} Casilla seleccionada: Slot #{}  ->  Coordenadas GML: tile_x = {}, tile_y = {}",
                            ic::CHECK_CIRCLE,
                            self.project.custom_night.slot_index + 1,
                            tx,
                            ty
                        ))
                        .color(Color32::from_rgb(90, 215, 130))
                        .strong(),
                    );
                }
                GridSubView::FourLayersDownSelector => {
                    cols[0].heading(format!("{}  Visor de Selección (4 Layers Down / Endless)", ic::GAME_CONTROLLER));
                    cols[0].label("Así se visualiza el animatrónico en el selector de la noche:");
                    cols[0].add_space(4.0);

                    egui::Frame::group(cols[0].style())
                        .fill(Color32::from_rgb(16, 20, 28))
                        .stroke(Stroke::new(1.5_f32, Color32::from_rgb(220, 160, 40)))
                        .inner_margin(12.0)
                        .show(&mut cols[0], |ui| {
                            ui.horizontal(|ui| {
                                // Tile 48x48
                                egui::Frame::none()
                                    .fill(Color32::BLACK)
                                    .stroke(Stroke::new(1.5_f32, Color32::WHITE))
                                    .inner_margin(2.0)
                                    .show(ui, |ui| {
                                        if let Some(tex) = &icon_tex {
                                            ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(48.0, 48.0)));
                                        } else {
                                            ui.allocate_space(Vec2::new(48.0, 48.0));
                                        }
                                    });

                                ui.add_space(10.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&self.project.gallery_info.full_name).size(16.0).strong().color(Color32::WHITE));
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("IA en 4LD: {}", self.project.behavior.four_layers_down_ai)).color(Color32::from_rgb(255, 200, 80)).strong());
                                        if ui.button("-").clicked() && self.project.behavior.four_layers_down_ai > 0 {
                                            self.project.behavior.four_layers_down_ai -= 1;
                                        }
                                        if ui.button("+").clicked() && self.project.behavior.four_layers_down_ai < 20 {
                                            self.project.behavior.four_layers_down_ai += 1;
                                        }
                                        if ui.button("20 Max").clicked() {
                                            self.project.behavior.four_layers_down_ai = 20;
                                        }
                                    });
                                    ui.label(RichText::new(format!("Debut: {}  |  Letalidad: Moderada", self.project.gallery_info.debut)).size(11.5).color(Color32::from_gray(170)));
                                });
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(RichText::new("Caja de Mecánica (draw_mechanic_box al pasar el mouse):").strong().color(Color32::from_rgb(180, 200, 240)));
                            egui::Frame::group(ui.style())
                                .fill(Color32::from_rgb(198, 198, 198))
                                .stroke(Stroke::new(1.5_f32, Color32::BLACK))
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&self.project.custom_night.cursor_tooltip).color(Color32::BLACK).size(12.0));
                                });
                        });
                }
                GridSubView::PauseMenuPreview => {
                    cols[0].heading(format!("{}  Visor del Menú de Pausa (In-Game ESC)", ic::PAUSE));
                    cols[0].label("Así se ve la ficha y descripción cuando el jugador pausa la noche:");
                    cols[0].add_space(4.0);

                    egui::Frame::group(cols[0].style())
                        .fill(Color32::from_rgb(14, 15, 20))
                        .stroke(Stroke::new(1.5_f32, Color32::from_gray(100)))
                        .inner_margin(12.0)
                        .show(&mut cols[0], |ui| {
                            ui.label(RichText::new("GAME PAUSED — CLICK TO UNPAUSE").strong().size(14.0).color(Color32::from_rgb(255, 215, 60)));
                            ui.add_space(6.0);

                            egui::Frame::group(ui.style())
                                .fill(Color32::from_rgb(190, 190, 190))
                                .stroke(Stroke::new(2.0_f32, Color32::BLACK))
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if let Some(tex) = &icon_tex {
                                            ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(48.0, 48.0)));
                                        }
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&self.project.gallery_info.full_name).size(15.0).strong().color(Color32::BLACK));
                                            ui.label(RichText::new(format!("Ubicación: {}", self.project.behavior.archetype.display_name())).size(11.0).color(Color32::from_rgb(40, 40, 40)));
                                        });
                                    });
                                    ui.add_space(6.0);
                                    ui.label(RichText::new(&self.project.custom_night.cursor_tooltip).color(Color32::BLACK).size(12.0));
                                });

                            ui.add_space(6.0);
                            ui.label(RichText::new("Items & Mejoras en inventario: AR Mask, Snowcone, Taser (3), Death Coin").size(11.0).color(Color32::from_gray(160)));
                        });
                }
            }

            // Columna Derecha: Parámetros de IA, 4 Layers Down y Textos
            cols[1].heading(format!("{}  Parámetros de IA & Textos", ic::SLIDERS_HORIZONTAL));
            cols[1].add_space(4.0);

            egui::ComboBox::from_label("Arquetipo de Comportamiento")
                .selected_text(self.project.behavior.archetype.display_name())
                .show_ui(&mut cols[1], |ui| {
                    for arch in AnimatronicArchetype::all() {
                        ui.selectable_value(&mut self.project.behavior.archetype, arch, arch.display_name());
                    }
                });
            egui::ComboBox::from_label("Contra-medida de Defensa")
                .selected_text(self.project.behavior.countermeasure.display_name())
                .show_ui(&mut cols[1], |ui| {
                    for cm in CountermeasureType::all() {
                        ui.selectable_value(&mut self.project.behavior.countermeasure, cm, cm.display_name());
                    }
                });
            cols[1].separator();
            cols[1].add(
                egui::Slider::new(&mut self.project.behavior.endless_mode_ai, 1..=20)
                    .text("IA Automática en Modo Endless"),
            );
            cols[1].add(
                egui::Slider::new(&mut self.project.behavior.four_layers_down_ai, 1..=20)
                    .text("IA Automática en Four Layers Down"),
            );
            cols[1].add(
                egui::Slider::new(&mut self.project.behavior.attack_window_frames_ai20, 120..=600)
                    .text("Frames para Atacar en IA 20 (320 = 5.3s)"),
            );

            cols[1].add_space(4.0);
            cols[1].label(RichText::new("Descripción emergente (Tooltip):").strong());
            cols[1].add(
                egui::TextEdit::multiline(&mut self.project.custom_night.cursor_tooltip)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );

            cols[1].add_space(4.0);
            cols[1].label(RichText::new("Consejo en Game Over:").strong());
            cols[1].add(
                egui::TextEdit::multiline(&mut self.project.custom_night.gameover_tip)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
        });
    }

    // --------------------------------------------------------------
    // WIDGETS AUXILIARES
    // --------------------------------------------------------------
    fn asset_card_row(
        ui: &mut egui::Ui,
        title: &str,
        subtitle: &str,
        target: &mut Option<PathBuf>,
        thumb: Option<TextureHandle>,
    ) {
        let exists = target.as_ref().map(|p| p.exists()).unwrap_or(false);
        let border_col = if exists {
            Color32::from_rgb(55, 140, 80)
        } else {
            Color32::from_rgb(70, 75, 88)
        };

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(28, 31, 38))
            .stroke(Stroke::new(1.0_f32, border_col))
            .rounding(Rounding::same(6.0))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(16, 18, 22))
                        .rounding(Rounding::same(4.0))
                        .show(ui, |ui| {
                            if let Some(tex) = thumb {
                                ui.add(egui::Image::new(&tex).fit_to_exact_size(Vec2::new(42.0, 42.0)));
                            } else {
                                ui.allocate_ui(Vec2::new(42.0, 42.0), |ui| {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(RichText::new(ic::IMAGE).size(18.0).color(Color32::DARK_GRAY));
                                    });
                                });
                            }
                        });

                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(title).strong().color(Color32::WHITE));
                        ui.label(RichText::new(subtitle).size(11.0).color(Color32::from_gray(165)));
                        let file_label = target
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| "No asignado".to_string());
                        let status_ico = if exists { ic::CHECK_CIRCLE } else { ic::WARNING_CIRCLE };
                        let status_col = if exists {
                            Color32::from_rgb(80, 210, 120)
                        } else {
                            Color32::from_rgb(230, 160, 60)
                        };
                        ui.label(RichText::new(format!("{} {}", status_ico, file_label)).size(11.5).color(status_col));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{} Examinar", ic::FOLDER_OPEN)).clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png", "jpg"]).pick_file() {
                                *target = Some(path);
                            }
                        }
                    });
                });
            });
    }

    fn audio_card_row(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        subtitle: &str,
        current_path: Option<PathBuf>,
        slot_id: u8,
    ) {
        let exists = current_path.as_ref().map(|p| p.exists()).unwrap_or(false);
        let border_col = if exists {
            Color32::from_rgb(55, 140, 80)
        } else {
            Color32::from_rgb(70, 75, 88)
        };

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(28, 31, 38))
            .stroke(Stroke::new(1.0_f32, border_col))
            .rounding(Rounding::same(6.0))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ic::SPEAKER_HIGH).size(22.0).color(Color32::from_rgb(240, 160, 55)));
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(title).strong().color(Color32::WHITE));
                        ui.label(RichText::new(subtitle).size(11.0).color(Color32::from_gray(160)));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(ic::FOLDER_OPEN).clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("OGG/WAV", &["ogg", "wav"]).pick_file() {
                                match slot_id {
                                    1 => self.project.sound_knock_alert = Some(path),
                                    2 => self.project.sound_voice_line = Some(path),
                                    3 => self.project.sound_defeat = Some(path),
                                    _ => self.project.sound_jumpscare = Some(path),
                                }
                            }
                        }
                        if exists {
                            if ui.button(ic::STOP).clicked() {
                                self.stop_audio();
                            }
                            if ui.button(ic::PLAY).clicked() {
                                if let Some(p) = &current_path {
                                    self.play_audio(p);
                                }
                            }
                        }
                    });
                });
            });
    }

    fn compact_folder_picker(ui: &mut egui::Ui, label: &str, target: &mut PathBuf) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).strong());
            let mut text = target.display().to_string();
            if ui.add(egui::TextEdit::singleline(&mut text).desired_width(150.0)).changed() {
                *target = PathBuf::from(text);
            }
            if ui.button(ic::FOLDER_OPEN).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    *target = p;
                }
            }
        });
    }

    // --------------------------------------------------------------
    // PESTAÑA 5: RUTEO DE CÁMARAS (ROOM 33: MAINLOCATIONMAP)
    // --------------------------------------------------------------
    fn ui_camera_map_router_tab(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Columna 1: Configuración de Ruta y Waypoints
            cols[0].heading(format!("{}  Ruteo de Cámaras (Room 33)", ic::MAP_TRIFOLD));
            cols[0].label(
                "Diseña la trayectoria que recorre el animatrónico a través del restaurante antes de llegar a la oficina."
            );
            cols[0].add_space(8.0);

            // Predefinidos de rutas de Five Nights at Frickbear's 3
            cols[0].horizontal(|ui| {
                if ui.button(format!("{} Staff Hallway (1->2->4)", ic::FLOW_ARROW)).clicked() {
                    self.project.advanced.camera_route = vec![
                        CameraRouteNode { camera_id: 1, camera_name: "CAM 01: Showroom".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 2, camera_name: "CAM 02: Dining Area".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 4, camera_name: "CAM 04: Staff Hallway".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                    ];
                }
                if ui.button(format!("{} Restroom Hallway (1->2->5)", ic::FLOW_ARROW)).clicked() {
                    self.project.advanced.camera_route = vec![
                        CameraRouteNode { camera_id: 1, camera_name: "CAM 01: Showroom".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 2, camera_name: "CAM 02: Dining Area".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 5, camera_name: "CAM 05: Restroom Hallway".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                    ];
                }
                if ui.button(format!("{} Entrada Principal (1->2->6)", ic::FLOW_ARROW)).clicked() {
                    self.project.advanced.camera_route = vec![
                        CameraRouteNode { camera_id: 1, camera_name: "CAM 01: Showroom".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 2, camera_name: "CAM 02: Dining Area".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 6, camera_name: "CAM 06: Entrance".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                    ];
                }
                if ui.button(format!("{} Limpiar", ic::TRASH)).clicked() {
                    self.project.advanced.camera_route.clear();
                }
            });

            cols[0].add_space(8.0);
            cols[0].separator();
            cols[0].add_space(6.0);

            // Lista de waypoints activos
            let mut to_remove_node = None;
            egui::ScrollArea::vertical().id_salt("cam_waypoints_scroll").max_height(260.0).show(&mut cols[0], |ui| {
                if self.project.advanced.camera_route.is_empty() {
                    ui.label(RichText::new("No hay cámaras en la ruta. Haz clic en las cámaras del plano derecho o usa un botón predefinido.").color(Color32::from_gray(140)).italics());
                } else {
                    for (step, node) in self.project.advanced.camera_route.iter_mut().enumerate() {
                        egui::Frame::group(ui.style())
                            .fill(Color32::from_rgb(26, 29, 36))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 78)))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let badge_col = Color32::from_rgb(230, 130, 30);
                                    ui.label(RichText::new(format!("#{}", step + 1)).strong().color(badge_col).size(15.0));
                                    ui.add_space(4.0);
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&node.camera_name).strong().color(Color32::WHITE));
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Velocidad:").size(11.0).color(Color32::from_gray(160)));
                                            ui.add(egui::Slider::new(&mut node.transition_time_modifier, 0.5..=3.0).text("x").show_value(true));
                                        });
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button(ic::TRASH).clicked() {
                                            to_remove_node = Some(step);
                                        }
                                        if ui.button(format!("{} Sprite", ic::IMAGE)).clicked() {
                                            if let Some(p) = rfd::FileDialog::new().add_filter("PNG", &["png"]).pick_file() {
                                                node.sprite_path = Some(p);
                                            }
                                        }
                                    });
                                });
                            });
                        ui.add_space(4.0);
                    }
                }
            });

            if let Some(idx) = to_remove_node {
                self.project.advanced.camera_route.remove(idx);
            }

            cols[0].add_space(8.0);
            cols[0].separator();
            cols[0].add_space(4.0);
            cols[0].label(RichText::new("Código GML Generado en Tiempo Real:").strong());
            let cam_ids: Vec<String> = self.project.advanced.camera_route.iter().map(|n| n.camera_id.to_string()).collect();
            let mut preview_code = format!(
                "// Array de movimiento en cámaras (Frickbear's 3)\n\
                 global.{id}_CameraRoute = [{cams}];\n\
                 global.{id}_RouteStep = 0;\n\
                 global.{id}_NextCamTimer = round(180 * {speed:.1});",
                id = self.project.internal_id,
                cams = cam_ids.join(", "),
                speed = self.project.advanced.camera_route.first().map(|n| n.transition_time_modifier).unwrap_or(1.0)
            );
            cols[0].add(egui::TextEdit::multiline(&mut preview_code).desired_rows(4).desired_width(f32::INFINITY));

            // Columna 2: Plano Topológico Interactivo de Cámaras
            cols[1].heading(format!("{}  Plano Topológico del Restaurante (Frickbear's 3)", ic::BUILDING));
            cols[1].label("Haz clic en cualquier cámara para añadirla o retirarla de la ruta:");
            cols[1].add_space(8.0);

            let (rect, _resp) = cols[1].allocate_exact_size(Vec2::new(540.0, 360.0), Sense::click());
            let painter = cols[1].painter_at(rect);

            // Fondo del mapa
            painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 22, 28));
            painter.rect_stroke(rect, 6.0, Stroke::new(1.5_f32, Color32::from_rgb(45, 55, 70)));

            // Habitaciones esquemáticas oficiales de Frickbear's 3
            let draw_room = |p: &egui::Painter, r: Rect, title: &str| {
                p.rect_filled(r, 4.0, Color32::from_rgb(24, 30, 40));
                p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(55, 68, 88)));
                p.text(Pos2::new(r.center().x, r.min.y + 10.0), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(11.0), Color32::from_gray(160));
            };

            let ox = rect.min.x;
            let oy = rect.min.y;

            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 180.0, oy + 20.0), Vec2::new(170.0, 65.0)), "SHOWROOM");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 160.0, oy + 100.0), Vec2::new(210.0, 95.0)), "DINING AREA");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 20.0, oy + 20.0), Vec2::new(130.0, 75.0)), "PIRATE'S COVE");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 20.0, oy + 115.0), Vec2::new(130.0, 85.0)), "ARCADE AREA");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 20.0, oy + 220.0), Vec2::new(130.0, 75.0)), "MAIN ENTRANCE");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 390.0, oy + 20.0), Vec2::new(135.0, 75.0)), "PARTS & SERVICE");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 390.0, oy + 115.0), Vec2::new(135.0, 85.0)), "STAFF HALLWAY");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 390.0, oy + 220.0), Vec2::new(135.0, 75.0)), "RESTROOM HALLWAY");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 195.0, oy + 220.0), Vec2::new(140.0, 75.0)), "SECURITY OFFICE");

            // Definición de las 8 cámaras oficiales de Frickbear's 3
            let cameras = [
                (1u8, "CAM 01", Pos2::new(ox + 265.0, oy + 58.0), "CAM 01: Showroom"),
                (2u8, "CAM 02", Pos2::new(ox + 265.0, oy + 150.0), "CAM 02: Dining Area"),
                (3u8, "CAM 03", Pos2::new(ox + 85.0, oy + 60.0), "CAM 03: Pirate's Cove"),
                (4u8, "CAM 04", Pos2::new(ox + 455.0, oy + 158.0), "CAM 04: Staff Hallway"),
                (5u8, "CAM 05", Pos2::new(ox + 455.0, oy + 258.0), "CAM 05: Restroom Hallway"),
                (6u8, "CAM 06", Pos2::new(ox + 85.0, oy + 258.0), "CAM 06: Entrance"),
                (7u8, "CAM 07", Pos2::new(ox + 85.0, oy + 158.0), "CAM 07: Arcade"),
                (8u8, "CAM 08", Pos2::new(ox + 455.0, oy + 60.0), "CAM 08: Parts & Service"),
            ];

            // Dibujar líneas conectoras de la ruta
            let route_cam_ids: Vec<u8> = self.project.advanced.camera_route.iter().map(|n| n.camera_id).collect();
            for w in 0..route_cam_ids.len().saturating_sub(1) {
                let id1 = route_cam_ids[w];
                let id2 = route_cam_ids[w + 1];
                if let (Some(c1), Some(c2)) = (cameras.iter().find(|c| c.0 == id1), cameras.iter().find(|c| c.0 == id2)) {
                    painter.line_segment([c1.2, c2.2], Stroke::new(2.5_f32, Color32::from_rgb(235, 125, 25)));
                }
            }

            // Si hay ruta, trazar línea final desde el último punto hacia la oficina
            if let Some(last_id) = route_cam_ids.last() {
                if let Some(c) = cameras.iter().find(|cm| cm.0 == *last_id) {
                    painter.line_segment([c.2, Pos2::new(ox + 265.0, oy + 250.0)], Stroke::new(2.0_f32, Color32::from_rgb(230, 60, 60)));
                }
            }

            let mut to_toggle_camera = None;

            // Dibujar cada nodo de cámara
            for (cid, cname, pos, full_desc) in cameras.iter() {
                let is_in_route = route_cam_ids.iter().position(|id| *id == *cid);
                let cam_rect = Rect::from_center_size(*pos, Vec2::new(62.0, 26.0));

                let (bg_col, border_col, txt_col) = if is_in_route.is_some() {
                    (Color32::from_rgb(180, 85, 15), Color32::from_rgb(255, 180, 50), Color32::WHITE)
                } else {
                    (Color32::from_rgb(32, 38, 50), Color32::from_rgb(70, 80, 100), Color32::from_gray(190))
                };

                painter.rect_filled(cam_rect, 4.0, bg_col);
                painter.rect_stroke(cam_rect, 4.0, Stroke::new(1.2_f32, border_col));

                let label_text = if let Some(order) = is_in_route {
                    format!("{} (#{})", cname, order + 1)
                } else {
                    cname.to_string()
                };

                painter.text(cam_rect.center(), egui::Align2::CENTER_CENTER, label_text, egui::FontId::proportional(11.5), txt_col);

                // Interacción de clic en el nodo
                let node_id = egui::Id::new(format!("cam_node_{}", cid));
                let interact = cols[1].interact(cam_rect, node_id, Sense::click());
                if interact.clicked() {
                    to_toggle_camera = Some((*cid, full_desc.to_string()));
                }
            }

            if let Some((cid, full_desc)) = to_toggle_camera {
                if let Some(pos_idx) = self.project.advanced.camera_route.iter().position(|n| n.camera_id == cid) {
                    self.project.advanced.camera_route.remove(pos_idx);
                } else {
                    self.project.advanced.camera_route.push(CameraRouteNode {
                        camera_id: cid,
                        camera_name: full_desc,
                        sprite_path: None,
                        transition_time_modifier: 1.0,
                    });
                }
            }
        });
    }

    // --------------------------------------------------------------
    // PESTAÑA 6: VISOR & DEBUGGER DE LOCACIONES 3D (MINIGAMEMAZE 1 A 5)
    // --------------------------------------------------------------
    // PESTAÑA 6: VISOR DE LOCACIONES 3D & DEBUGGER (MINIGAMEMAZE 1 A 5)
    // --------------------------------------------------------------
    fn ui_environment_3d_builder_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let cur_room_name = self.selected_salvage_location.room_name();

        // ==============================================================
        // BARRA SUPERIOR INTUITIVA: MODOS, PAUSA/REINICIO Y SELECTOR DE SALAS
        // ==============================================================
        ui.horizontal(|ui| {
            // Modos de Vista
            let is_sim = self.salvage_tab_mode == SalvageTabMode::LiveSimulator;
            if ui.selectable_label(is_sim, RichText::new(format!("{}  Simulador 3D (WASD)", ic::GAME_CONTROLLER)).strong().size(13.0)).clicked() {
                self.salvage_tab_mode = SalvageTabMode::LiveSimulator;
            }
            let is_bp = self.salvage_tab_mode == SalvageTabMode::BlueprintEditor;
            if ui.selectable_label(is_bp, RichText::new(format!("{}  Plano Cenital & Debugger (2D)", ic::COMPASS)).strong().size(13.0)).clicked() {
                self.salvage_tab_mode = SalvageTabMode::BlueprintEditor;
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Botones de Simulación: Pausar, Reiniciar y Linterna
            let (pause_txt, pause_col) = if self.salvage_sim_paused {
                (format!("{} Reanudar", ic::PLAY), Color32::from_rgb(90, 225, 120))
            } else {
                (format!("{} Pausar", ic::PAUSE), Color32::from_rgb(255, 180, 50))
            };

            if ui.button(RichText::new(pause_txt).strong().color(pause_col)).clicked() {
                self.salvage_sim_paused = !self.salvage_sim_paused;
            }

            if ui.button(format!("{} Reiniciar", ic::ARROW_COUNTER_CLOCKWISE)).clicked() {
                self.salvage_cam_distance = 0.0;
                self.salvage_cam_yaw = 0.0;
                self.salvage_sim_paused = false;
            }

            let light_col = if self.salvage_flashlight { Color32::from_rgb(255, 230, 90) } else { Color32::GRAY };
            if ui.button(RichText::new(format!("{} Linterna", ic::FLASHLIGHT)).strong().color(light_col)).clicked() {
                self.salvage_flashlight = !self.salvage_flashlight;
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            // Selector horizontal limpio de las 5 Locaciones Oficiales de Frickbear's 3
            for loc in SalvageLocationIndex::all() {
                let is_sel = *loc == self.selected_salvage_location;
                let short_name = match loc {
                    SalvageLocationIndex::Location1Fnaf1 => "1. FNAF 1 (1993)",
                    SalvageLocationIndex::Location2Fnaf2 => "2. FNAF 2 (1987)",
                    SalvageLocationIndex::Location3Fnaf3 => "3. FNAF 3 (Fright)",
                    SalvageLocationIndex::Location4WilliamWoods => "4. Woods",
                    SalvageLocationIndex::Location5SisterLocation => "5. Sister Loc",
                };
                if ui.selectable_label(is_sel, RichText::new(short_name).size(12.0)).clicked() {
                    self.selected_salvage_location = *loc;
                    self.project.salvage_placement.target_room = loc.room_name().to_string();
                    self.salvage_cam_distance = 0.0;
                    self.salvage_cam_yaw = 0.0;
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // Captura fluida de teclas WASD / Flechas para navegación 3D
        if self.salvage_tab_mode == SalvageTabMode::LiveSimulator && !self.salvage_sim_paused {
            ctx.input(|i| {
                let speed = 2.4;
                let turn_speed = 1.6;
                if i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp) {
                    self.salvage_cam_distance = (self.salvage_cam_distance + speed).clamp(-120.0, 350.0);
                }
                if i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown) {
                    self.salvage_cam_distance = (self.salvage_cam_distance - speed).clamp(-120.0, 350.0);
                }
                if i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft) {
                    self.salvage_cam_yaw = (self.salvage_cam_yaw - turn_speed).clamp(-45.0, 45.0);
                }
                if i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight) {
                    self.salvage_cam_yaw = (self.salvage_cam_yaw + turn_speed).clamp(-45.0, 45.0);
                }
                if i.key_pressed(egui::Key::Space) {
                    self.salvage_flashlight = !self.salvage_flashlight;
                }
            });
        }

        if self.salvage_tab_mode == SalvageTabMode::LiveSimulator {
            let t = if self.salvage_sim_paused { 0.5 } else { self.sim_time };

            // ==============================================================
            // VIEWPORT 3D PRINCIPAL (EN PRIMERA PERSONA CON TEXTURAS AMBIENTALES)
            // ==============================================================
            let avail_w = ui.available_width().max(600.0);
            let view_h = 400.0;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, view_h), Sense::click());
            let painter = ui.painter_at(rect);

            // Paletas de color ambiental por locación
            let (bg_col, floor_col, wall_col, brick_col) = match self.selected_salvage_location {
                SalvageLocationIndex::Location1Fnaf1 => (
                    Color32::from_rgb(10, 8, 7),
                    Color32::from_rgb(20, 16, 14),
                    Color32::from_rgb(38, 22, 16),
                    Color32::from_rgb(90, 35, 25),
                ),
                SalvageLocationIndex::Location2Fnaf2 => (
                    Color32::from_rgb(14, 10, 22),
                    Color32::from_rgb(24, 18, 36),
                    Color32::from_rgb(46, 26, 62),
                    Color32::from_rgb(60, 30, 80),
                ),
                SalvageLocationIndex::Location3Fnaf3 => (
                    Color32::from_rgb(6, 14, 8),
                    Color32::from_rgb(16, 24, 18),
                    Color32::from_rgb(22, 42, 24),
                    Color32::from_rgb(30, 58, 32),
                ),
                SalvageLocationIndex::Location4WilliamWoods => (
                    Color32::from_rgb(5, 8, 14),
                    Color32::from_rgb(10, 15, 20),
                    Color32::from_rgb(14, 22, 30),
                    Color32::from_rgb(18, 28, 40),
                ),
                SalvageLocationIndex::Location5SisterLocation => (
                    Color32::from_rgb(6, 12, 22),
                    Color32::from_rgb(14, 20, 32),
                    Color32::from_rgb(22, 36, 54),
                    Color32::from_rgb(30, 48, 70),
                ),
            };

            // 1. Fondo base
            painter.rect_filled(rect, 6.0, bg_col);

            // 2. Geometría 3D en perspectiva de pasillo con rotación yaw y profundidad
            let yaw_offset = self.salvage_cam_yaw * 4.0;
            let vanishing_pt = Pos2::new(rect.center().x + yaw_offset, rect.center().y - 12.0);
            let horizon_w = (65.0 + (self.salvage_cam_distance * 0.22)).clamp(35.0, 160.0);
            let horizon_h = (45.0 + (self.salvage_cam_distance * 0.16)).clamp(25.0, 120.0);
            let back_wall = Rect::from_center_size(vanishing_pt, Vec2::new(horizon_w, horizon_h));

            // Pared del fondo
            painter.rect_filled(back_wall, 2.0, wall_col);
            painter.rect_stroke(back_wall, 2.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 25)));

            // Suelo en perspectiva
            let floor_poly = vec![
                Pos2::new(back_wall.min.x, back_wall.max.y),
                Pos2::new(back_wall.max.x, back_wall.max.y),
                rect.right_bottom(),
                rect.left_bottom(),
            ];
            painter.add(egui::Shape::convex_polygon(floor_poly, floor_col, Stroke::NONE));

            // Pared Izquierda
            let left_wall = vec![
                rect.left_top(),
                Pos2::new(back_wall.min.x, back_wall.min.y),
                Pos2::new(back_wall.min.x, back_wall.max.y),
                rect.left_bottom(),
            ];
            painter.add(egui::Shape::convex_polygon(left_wall, wall_col, Stroke::NONE));

            // Pared Derecha
            let right_wall = vec![
                Pos2::new(back_wall.max.x, back_wall.min.y),
                rect.right_top(),
                rect.right_bottom(),
                Pos2::new(back_wall.max.x, back_wall.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(right_wall, wall_col, Stroke::NONE));

            // ==========================================================
            // TEXTURIZADO Y DETALLES EN PERSPECTIVA 3D DE CADA SALA
            // ==========================================================
            match self.selected_salvage_location {
                SalvageLocationIndex::Location1Fnaf1 | SalvageLocationIndex::Location2Fnaf2 => {
                    // SUELO: Tablero ajedrezado (Checkerboard tiles) con baldosas alternadas
                    let num_tiles_x = 8;
                    let num_tiles_y = 6;
                    for iy in 0..num_tiles_y {
                        let f0 = (iy as f32 / num_tiles_y as f32).powf(1.6);
                        let f1 = ((iy + 1) as f32 / num_tiles_y as f32).powf(1.6);
                        let y0 = back_wall.max.y + ((rect.max.y - back_wall.max.y) * f0);
                        let y1 = back_wall.max.y + ((rect.max.y - back_wall.max.y) * f1);

                        let x_l0 = rect.min.x + ((back_wall.min.x - rect.min.x) * (1.0 - f0));
                        let x_r0 = rect.max.x - ((rect.max.x - back_wall.max.x) * (1.0 - f0));
                        let x_l1 = rect.min.x + ((back_wall.min.x - rect.min.x) * (1.0 - f1));
                        let x_r1 = rect.max.x - ((rect.max.x - back_wall.max.x) * (1.0 - f1));

                        for ix in 0..num_tiles_x {
                            let fx0 = ix as f32 / num_tiles_x as f32;
                            let fx1 = (ix + 1) as f32 / num_tiles_x as f32;
                            let p_top_left = Pos2::new(x_l0 + ((x_r0 - x_l0) * fx0), y0);
                            let p_top_right = Pos2::new(x_l0 + ((x_r0 - x_l0) * fx1), y0);
                            let p_bot_right = Pos2::new(x_l1 + ((x_r1 - x_l1) * fx1), y1);
                            let p_bot_left = Pos2::new(x_l1 + ((x_r1 - x_l1) * fx0), y1);

                            let is_white = (ix + iy) % 2 == 0;
                            let tile_color = if is_white {
                                Color32::from_rgba_unmultiplied(220, 220, 225, 45)
                            } else {
                                Color32::from_rgba_unmultiplied(15, 15, 20, 70)
                            };
                            painter.add(egui::Shape::convex_polygon(
                                vec![p_top_left, p_top_right, p_bot_right, p_bot_left],
                                tile_color,
                                Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 50)),
                            ));
                        }
                    }

                    // PAREDES: Zócalos de madera y líneas de ladrillo (sMazeTex_Wood)
                    for side in [0, 1] {
                        for row in 0..5 {
                            let f = (row as f32 / 5.0).powf(1.1);
                            let y_start = if side == 0 { rect.left_top().y } else { rect.right_top().y };
                            let y_end = if side == 0 { rect.left_bottom().y } else { rect.right_bottom().y };
                            let wall_y = y_start + ((y_end - y_start) * (0.35 + (f * 0.55)));
                            let back_y = back_wall.min.y + (back_wall.height() * (0.35 + (f * 0.55)));

                            let p0 = if side == 0 { Pos2::new(rect.left_top().x, wall_y) } else { Pos2::new(rect.right_top().x, wall_y) };
                            let p1 = if side == 0 { Pos2::new(back_wall.min.x, back_y) } else { Pos2::new(back_wall.max.x, back_y) };
                            painter.line_segment([p0, p1], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(brick_col.r(), brick_col.g(), brick_col.b(), 70)));
                        }
                    }

                    // TECHO: Lámpara fluorescente rectangular
                    let lamp_rect = Rect::from_min_max(
                        Pos2::new(vanishing_pt.x - 22.0, rect.min.y + 10.0),
                        Pos2::new(vanishing_pt.x + 22.0, back_wall.min.y - 10.0),
                    );
                    painter.rect_filled(lamp_rect, 2.0, Color32::from_rgba_unmultiplied(240, 240, 220, 60));
                    painter.rect_stroke(lamp_rect, 2.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 90)));
                }

                SalvageLocationIndex::Location3Fnaf3 => {
                    // SUELO: Planchas de hormigón desgastado con grietas oscuras (sMazeTex_FloorConcrete)
                    for iy in 1..=5 {
                        let f = (iy as f32 / 5.0).powf(1.5);
                        let y = back_wall.max.y + ((rect.max.y - back_wall.max.y) * f);
                        let x_l = rect.min.x + ((back_wall.min.x - rect.min.x) * (1.0 - f));
                        let x_r = rect.max.x - ((rect.max.x - back_wall.max.x) * (1.0 - f));
                        painter.line_segment([Pos2::new(x_l, y), Pos2::new(x_r, y)], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(10, 20, 12, 180)));
                    }

                    // TECHO & CONDUCTOS: Tuberías cilíndricas 3D de Fazbear's Fright (sMazeTex_Vent)
                    let pipe_p0 = Pos2::new(rect.min.x + 40.0, rect.min.y + 20.0);
                    let pipe_p1 = Pos2::new(back_wall.min.x + 8.0, back_wall.min.y + 4.0);
                    painter.line_segment([pipe_p0, pipe_p1], Stroke::new(6.0_f32, Color32::from_rgb(25, 45, 28)));
                    painter.line_segment([pipe_p0, pipe_p1], Stroke::new(2.0_f32, Color32::from_rgba_unmultiplied(120, 210, 130, 80)));

                    // Cables colgantes
                    let cable_mid = Pos2::new(vanishing_pt.x, back_wall.min.y - 6.0 + ((t * 2.0).sin() * 2.0));
                    painter.line_segment([Pos2::new(rect.left_top().x + 80.0, rect.min.y + 30.0), cable_mid], Stroke::new(1.5_f32, Color32::from_rgb(18, 30, 20)));
                    painter.line_segment([cable_mid, Pos2::new(rect.right_top().x - 80.0, rect.min.y + 30.0)], Stroke::new(1.5_f32, Color32::from_rgb(18, 30, 20)));
                }

                SalvageLocationIndex::Location4WilliamWoods => {
                    // BOSQUE 3D: Siluetas de árboles y lluvia continua
                    for tree in 0..6 {
                        let tf = tree as f32 / 6.0;
                        let tx_left = rect.min.x + (60.0 * (1.0 - tf));
                        let tx_right = rect.max.x - (60.0 * (1.0 - tf));
                        let ty_top = rect.min.y + 40.0 + (tf * 80.0);
                        let ty_bot = rect.max.y - (tf * 60.0);

                        painter.line_segment([Pos2::new(tx_left, ty_top), Pos2::new(tx_left, ty_bot)], Stroke::new(8.0_f32 * (1.0 - tf), Color32::from_rgb(10, 16, 22)));
                        painter.line_segment([Pos2::new(tx_right, ty_top), Pos2::new(tx_right, ty_bot)], Stroke::new(8.0_f32 * (1.0 - tf), Color32::from_rgb(10, 16, 22)));
                    }

                    // Gotas de lluvia animadas
                    for r in 0..24 {
                        let rx = rect.min.x + (((r as f32 * 41.0) + (t * 280.0)) % rect.width());
                        let ry = rect.min.y + (((r as f32 * 67.0) + (t * 550.0)) % rect.height());
                        painter.line_segment([Pos2::new(rx, ry), Pos2::new(rx - 2.5, ry + 14.0)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(180, 220, 255, 75)));
                    }
                }

                SalvageLocationIndex::Location5SisterLocation => {
                    // SUELO: Rejillas industriales con líneas de neón azul
                    for j in 1..=5 {
                        let f = (j as f32 / 5.0).powf(1.5);
                        let y = back_wall.max.y + ((rect.max.y - back_wall.max.y) * f);
                        let x_l = rect.min.x + ((back_wall.min.x - rect.min.x) * (1.0 - f));
                        let x_r = rect.max.x - ((rect.max.x - back_wall.max.x) * (1.0 - f));
                        painter.line_segment([Pos2::new(x_l, y), Pos2::new(x_r, y)], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(50, 160, 240, 60)));
                    }

                    // Tiras de neón LED en las esquinas de pared
                    painter.line_segment([rect.left_bottom(), Pos2::new(back_wall.min.x, back_wall.max.y)], Stroke::new(2.5_f32, Color32::from_rgba_unmultiplied(70, 210, 255, 160)));
                    painter.line_segment([rect.right_bottom(), Pos2::new(back_wall.max.x, back_wall.max.y)], Stroke::new(2.5_f32, Color32::from_rgba_unmultiplied(70, 210, 255, 160)));
                }
            }

            // 3. Linterna dinámica
            if self.salvage_flashlight {
                let flicker = if self.salvage_sim_paused { 0.0 } else { ((t * 25.0).sin() * 2.0).abs() };
                let spot_center = vanishing_pt;
                painter.circle_filled(spot_center, 210.0 + flicker, Color32::from_rgba_unmultiplied(255, 240, 190, 22));
                painter.circle_filled(spot_center, 135.0 + flicker, Color32::from_rgba_unmultiplied(255, 245, 200, 36));
                painter.circle_filled(spot_center, 75.0 + (flicker * 0.5), Color32::from_rgba_unmultiplied(255, 250, 210, 52));
            } else {
                painter.rect_filled(rect, 6.0, Color32::from_rgba_unmultiplied(0, 0, 0, 225));
            }

            // 4. Renderizado del Animatrónico (SÓLO SI EL USUARIO LO ACTIVÓ EXPLÍCITAMENTE CON UN SPRITE)
            // CERO MURRAY MOOSE POR DEFECTO
            if self.project.salvage_placement.enabled {
                let sprite_opt = self.project.salvage_placement.salvage_sprite_path.clone();
                if let Some(p) = sprite_opt {
                    if let Some(tex) = self.get_or_load_tex(ctx, &p, 320) {
                        let breathe = if self.salvage_sim_paused { 0.0 } else { (t * 2.2).sin() * 3.0 };
                        let anim_scale = (1.0 + (self.salvage_cam_distance * 0.005)).clamp(0.4, 2.2);
                        let anim_center = Pos2::new(vanishing_pt.x, vanishing_pt.y + 15.0 + breathe);
                        let sz = Vec2::new(140.0 * anim_scale, 140.0 * anim_scale);
                        let s_rect = Rect::from_center_size(anim_center, sz);

                        if self.salvage_flashlight {
                            painter.image(tex.id(), s_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                            painter.text(
                                Pos2::new(anim_center.x, anim_center.y - (75.0 * anim_scale)),
                                egui::Align2::CENTER_CENTER,
                                format!("Animatrónico Custom: ({}, {})", self.project.salvage_placement.x, self.project.salvage_placement.y),
                                egui::FontId::proportional(11.0),
                                Color32::YELLOW,
                            );
                        } else {
                            painter.circle_filled(Pos2::new(anim_center.x - 14.0 * anim_scale, anim_center.y - 6.0), 4.5 * anim_scale, Color32::from_rgb(90, 255, 130));
                            painter.circle_filled(Pos2::new(anim_center.x + 14.0 * anim_scale, anim_center.y - 6.0), 4.5 * anim_scale, Color32::from_rgb(90, 255, 130));
                        }
                    }
                }
            }

            // HUD Translúcido sobre el Viewport 3D
            // Esquina Superior Izquierda: Sala actual e información
            let hud_top_left = Rect::from_min_size(Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0), Vec2::new(320.0, 48.0));
            painter.rect_filled(hud_top_left, 4.0, Color32::from_rgba_unmultiplied(10, 12, 16, 190));
            painter.rect_stroke(hud_top_left, 4.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 30)));
            painter.text(
                Pos2::new(hud_top_left.min.x + 8.0, hud_top_left.min.y + 8.0),
                egui::Align2::LEFT_TOP,
                format!("{} {}", ic::MAP_PIN, self.selected_salvage_location.display_name()),
                egui::FontId::proportional(12.5),
                Color32::from_rgb(255, 200, 80),
            );
            painter.text(
                Pos2::new(hud_top_left.min.x + 8.0, hud_top_left.min.y + 26.0),
                egui::Align2::LEFT_TOP,
                format!("BSO: {} | {}", self.selected_salvage_location.soundtrack_name(), cur_room_name),
                egui::FontId::proportional(10.5),
                Color32::from_gray(190),
            );

            // Esquina Inferior Izquierda: Guía de Navegación WASD
            let hud_bottom = Rect::from_min_size(Pos2::new(rect.min.x + 12.0, rect.max.y - 36.0), Vec2::new(450.0, 24.0));
            painter.rect_filled(hud_bottom, 4.0, Color32::from_rgba_unmultiplied(10, 12, 16, 190));
            painter.text(
                Pos2::new(hud_bottom.min.x + 8.0, hud_bottom.center().y),
                egui::Align2::LEFT_CENTER,
                format!("[W][A][S][D] / Flechas: Caminar y Rotar | [Espacio]: Linterna | Dist: {:.1}m | Yaw: {:.0}°", self.salvage_cam_distance, self.salvage_cam_yaw),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(140, 210, 255),
            );

            // Banner central si está pausado
            if self.salvage_sim_paused {
                let pause_banner = Rect::from_center_size(rect.center(), Vec2::new(260.0, 40.0));
                painter.rect_filled(pause_banner, 6.0, Color32::from_rgba_unmultiplied(20, 25, 35, 220));
                painter.rect_stroke(pause_banner, 6.0, Stroke::new(1.5_f32, Color32::from_rgb(255, 190, 60)));
                painter.text(
                    pause_banner.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{} SIMULACIÓN PAUSADA", ic::PAUSE),
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(255, 200, 80),
                );
            }

            if resp.clicked() {
                self.salvage_flashlight = !self.salvage_flashlight;
            }

            // ==============================================================
            // PANEL INFERIOR LIMPIO: HERRAMIENTA OPCIONAL PARA COLOCAR ANIMATRÓNICO
            // ==============================================================
            ui.add_space(8.0);
            egui::CollapsingHeader::new(RichText::new("⚙️  Herramienta Opcional: Colocar Animatrónico en Coordenadas (X, Y)").size(12.5))
                .default_open(false)
                .show(ui, |ui| {
                    ui.label("Esta herramienta te permite ubicar tu propio sprite de animatrónico en coordenadas exactas de la sala:");
                    ui.checkbox(&mut self.project.salvage_placement.enabled, "Activar Animatrónico en la Locación");

                    ui.horizontal(|ui| {
                        ui.label("Coordenada X:");
                        ui.add(egui::DragValue::new(&mut self.project.salvage_placement.x).speed(2.0));
                        ui.add_space(10.0);
                        ui.label("Coordenada Y:");
                        ui.add(egui::DragValue::new(&mut self.project.salvage_placement.y).speed(2.0));
                    });

                    ui.horizontal(|ui| {
                        if ui.button(format!("{} Seleccionar Sprite .PNG", ic::FOLDER_OPEN)).clicked() {
                            if let Some(file) = rfd::FileDialog::new()
                                .add_filter("Imágenes PNG", &["png"])
                                .set_directory(&self.project.workspace_dir)
                                .pick_file()
                            {
                                self.project.salvage_placement.salvage_sprite_path = Some(file);
                            }
                        }
                        if let Some(p) = &self.project.salvage_placement.salvage_sprite_path {
                            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("sprite.png");
                            ui.label(RichText::new(name).monospace().size(11.0).color(Color32::from_rgb(120, 220, 160)));
                            if ui.button(format!("{} Quitar", ic::TRASH)).clicked() {
                                self.project.salvage_placement.salvage_sprite_path = None;
                            }
                        } else {
                            ui.label(RichText::new("(Ningún sprite seleccionado)").size(11.0).color(Color32::GRAY));
                        }
                    });

                    let gml_code = format!(
                        "// Inyección en Room: {}\nvar _inst = instance_create_layer({}, {}, \"Instances\", oInteraction_Animatronic);",
                        cur_room_name,
                        self.project.salvage_placement.x,
                        self.project.salvage_placement.y
                    );
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Código GML:").strong());
                        if ui.button(format!("{} Copiar", ic::COPY)).clicked() {
                            ui.output_mut(|o| o.copied_text = gml_code.clone());
                            self.status_message = "Código GML copiado al portapapeles.".to_string();
                            self.status_is_error = false;
                        }
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut gml_code.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                    );
                });
        } else {
            // ==============================================================
            // MODO 2: DEBUGGER DE PLANO CENITAL REAL (INSTANCIAS GM 2D)
            // ==============================================================
            let active_room_data = self.maze_rooms.get(&self.project.salvage_placement.target_room).cloned();

            ui.columns(2, |cols| {
                // COLUMNA 1: FILTROS DE CAPAS, METADATOS Y ENTIDAD INSPECCIONADA
                cols[0].heading(format!("{}  Debugger de Sala & Entidades", ic::BUG));
                cols[0].label("Inspección de las instancias reales extraídas del motor para las salas de laberinto:");
                cols[0].add_space(8.0);

                // Selector de Sala Destino con id_salt
                cols[0].horizontal(|ui| {
                    ui.label(RichText::new("Sala a Inspeccionar:").strong());
                    egui::ComboBox::from_id_salt("maze_room_select")
                        .selected_text(&self.project.salvage_placement.target_room)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.project.salvage_placement.target_room, "MinigameMaze_1".to_string(), "MinigameMaze_1 (FNAF 1 - 292 insts)");
                            ui.selectable_value(&mut self.project.salvage_placement.target_room, "MinigameMaze_2".to_string(), "MinigameMaze_2 (FNAF 2 - 327 insts)");
                            ui.selectable_value(&mut self.project.salvage_placement.target_room, "MinigameMaze_3".to_string(), "MinigameMaze_3 (FNAF 3 - 289 insts)");
                            ui.selectable_value(&mut self.project.salvage_placement.target_room, "MinigameMaze_4".to_string(), "MinigameMaze_4 (William's Woods - 486 insts)");
                            ui.selectable_value(&mut self.project.salvage_placement.target_room, "MinigameMaze_5".to_string(), "MinigameMaze_5 (Sister Location - 313 insts)");
                            ui.selectable_value(&mut self.project.salvage_placement.target_room, "Fredbear_3DMaze".to_string(), "Fredbear_3DMaze (Secreto - 7 insts)");
                        });
                });

                cols[0].add_space(8.0);

                // Filtros de Capas
                cols[0].label(RichText::new("Filtros de Visibilidad en el Plano:").strong());
                cols[0].horizontal(|ui| {
                    ui.checkbox(&mut self.dbg_filter_walls, "Paredes");
                    ui.checkbox(&mut self.dbg_filter_props, "Props");
                    ui.checkbox(&mut self.dbg_filter_interactive, "Interacciones");
                    ui.checkbox(&mut self.dbg_filter_spawns, "Spawns");
                });

                cols[0].add_space(10.0);
                cols[0].separator();
                cols[0].add_space(6.0);

                // Información de la Entidad Seleccionada en el Plano
                cols[0].heading(format!("{} Datos de Instancia Seleccionada", ic::INFO));
                if let Some(inst_id) = self.dbg_selected_instance_id {
                    if let Some(r_data) = &active_room_data {
                        if let Some(inst) = r_data.instances.iter().find(|i| i.id == inst_id) {
                            egui::Frame::none()
                                .fill(Color32::from_rgb(22, 27, 36))
                                .rounding(Rounding::same(6.0))
                                .inner_margin(8.0)
                                .show(&mut cols[0], |ui| {
                                    ui.label(RichText::new(format!("Objeto: {}", inst.object)).strong().color(Color32::from_rgb(110, 220, 255)));
                                    ui.label(format!("Instance ID: {}", inst.id));
                                    ui.label(format!("Coordenadas: ({}, {})", inst.x, inst.y));
                                    ui.label(format!("Escala: ({:.1}, {:.1})", inst.sx, inst.sy));
                                });
                        } else {
                            cols[0].label("Instancia no encontrada en esta sala.");
                        }
                    }
                } else {
                    cols[0].label(RichText::new("Haz clic en cualquier entidad o prop en el plano derecho para ver sus coordenadas y metadatos.").color(Color32::GRAY));
                }

                cols[0].add_space(10.0);
                cols[0].separator();
                cols[0].add_space(6.0);

                // Coordenadas del Animatrónico Custom (si está activado)
                cols[0].heading(format!("{} Ubicación del Animatrónico en el Plano", ic::GHOST));
                cols[0].checkbox(&mut self.project.salvage_placement.enabled, "Mostrar Marcador en el Plano");
                cols[0].horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.project.salvage_placement.x).speed(2.0));
                    ui.add_space(8.0);
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.project.salvage_placement.y).speed(2.0));
                });

                if cols[0].button("Fijar en Spawn Predeterminado").clicked() {
                    let (x, y) = match self.project.salvage_placement.target_room.as_str() {
                        "MinigameMaze_1" => (1128, 96),
                        "MinigameMaze_2" => (1424, 56),
                        "MinigameMaze_3" => (1232, 128),
                        "MinigameMaze_4" => (1824, 48),
                        "MinigameMaze_5" => (1904, 72),
                        _ => (320, 180),
                    };
                    self.project.salvage_placement.x = x;
                    self.project.salvage_placement.y = y;
                }

                // ==============================================================
                // COLUMNA 2: PLANO CENITAL INTERACTIVO CON INSTANCIAS REALES
                // ==============================================================
                cols[1].heading(format!("{}  Plano Cenital Completo ({})", ic::MAP_TRIFOLD, self.project.salvage_placement.target_room));
                cols[1].label("Visualización a escala de las instancias reales de GameMaker. Clic para inspeccionar coordenadas:");
                cols[1].add_space(6.0);

                let (rect, resp) = cols[1].allocate_exact_size(Vec2::new(540.0, 380.0), Sense::click_and_drag());
                let painter = cols[1].painter_at(rect);

                // Fondo del canvas
                painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 15, 20));
                painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 75)));

                // Cuadrícula sutil
                for gx in (0..=540).step_by(27) {
                    let x = rect.min.x + gx as f32;
                    painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], Stroke::new(0.5_f32, Color32::from_rgb(20, 25, 35)));
                }
                for gy in (0..=380).step_by(25) {
                    let y = rect.min.y + gy as f32;
                    painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], Stroke::new(0.5_f32, Color32::from_rgb(20, 25, 35)));
                }

                if let Some(r_data) = &active_room_data {
                    let mut min_x = -100;
                    let mut max_x = 2200;
                    let mut min_y = -50;
                    let mut max_y = 1650;

                    for i in &r_data.instances {
                        if i.x < min_x { min_x = i.x; }
                        if i.x > max_x { max_x = i.x; }
                        if i.y < min_y { min_y = i.y; }
                        if i.y > max_y { max_y = i.y; }
                    }

                    let span_x = (max_x - min_x).max(1) as f32;
                    let span_y = (max_y - min_y).max(1) as f32;
                    let pad = 12.0;
                    let draw_w = rect.width() - (pad * 2.0);
                    let draw_h = rect.height() - (pad * 2.0);

                    let to_screen = |gx: i32, gy: i32| -> Pos2 {
                        let nx = (gx - min_x) as f32 / span_x;
                        let ny = (gy - min_y) as f32 / span_y;
                        Pos2::new(rect.min.x + pad + (nx * draw_w), rect.min.y + pad + (ny * draw_h))
                    };

                    let to_game = |pos: Pos2| -> (i32, i32) {
                        let nx = ((pos.x - (rect.min.x + pad)) / draw_w).clamp(0.0, 1.0);
                        let ny = ((pos.y - (rect.min.y + pad)) / draw_h).clamp(0.0, 1.0);
                        let gx = min_x + (nx * span_x) as i32;
                        let gy = min_y + (ny * span_y) as i32;
                        (gx, gy)
                    };

                    let mut clicked_inst_id = None;

                    // Dibujar instancias reales
                    for inst in &r_data.instances {
                        let is_wall = inst.object.contains("Wall") || inst.object.contains("Floor") || inst.object.contains("Ceiling");
                        let is_prop = inst.object.contains("Prop") || inst.object.contains("Sign");
                        let is_interactive = inst.object.contains("Escape") || inst.object.contains("Shop") || inst.object.contains("Camera") || inst.object.contains("Point");
                        let is_spawn = inst.object.contains("Animatronic") || inst.object.contains("Enemy");

                        if is_wall && !self.dbg_filter_walls { continue; }
                        if is_prop && !self.dbg_filter_props { continue; }
                        if is_interactive && !self.dbg_filter_interactive { continue; }
                        if is_spawn && !self.dbg_filter_spawns { continue; }

                        let sp = to_screen(inst.x, inst.y);

                        let (color, radius) = if is_spawn {
                            (Color32::from_rgb(255, 60, 60), 4.5)
                        } else if inst.object.contains("Camera") {
                            (Color32::from_rgb(80, 180, 255), 5.5)
                        } else if inst.object.contains("Escape") {
                            (Color32::from_rgb(60, 230, 110), 6.0)
                        } else if inst.object.contains("Shop") {
                            (Color32::from_rgb(255, 215, 60), 5.5)
                        } else if is_prop {
                            (Color32::from_rgb(180, 100, 240), 3.5)
                        } else {
                            (Color32::from_rgb(55, 65, 85), 2.5)
                        };

                        painter.circle_filled(sp, radius, color);

                        if Some(inst.id) == self.dbg_selected_instance_id {
                            painter.circle_stroke(sp, radius + 2.0, Stroke::new(1.5_f32, Color32::WHITE));
                        }

                        if resp.clicked() && resp.hover_pos().map(|hp| hp.distance(sp) < 8.0).unwrap_or(false) {
                            clicked_inst_id = Some(inst.id);
                        }
                    }

                    if let Some(id) = clicked_inst_id {
                        self.dbg_selected_instance_id = Some(id);
                    }

                    // Marcador del animatrónico (SÓLO SI ESTÁ ACTIVADO)
                    if self.project.salvage_placement.enabled {
                        let anim_screen_pos = to_screen(self.project.salvage_placement.x, self.project.salvage_placement.y);
                        let anim_marker_rect = Rect::from_center_size(anim_screen_pos, Vec2::new(22.0, 22.0));

                        painter.circle_filled(anim_screen_pos, 8.5, Color32::from_rgb(255, 200, 40));
                        painter.circle_stroke(anim_screen_pos, 10.0, Stroke::new(2.0_f32, Color32::WHITE));
                        painter.text(anim_screen_pos, egui::Align2::CENTER_CENTER, ic::GHOST, egui::FontId::proportional(11.0), Color32::BLACK);

                        // Arrastrar marcador con el ratón
                        let anim_drag_id = egui::Id::new("anim_marker_drag_2d");
                        let anim_interact = cols[1].interact(anim_marker_rect, anim_drag_id, Sense::drag());
                        if anim_interact.dragged() {
                            if let Some(mouse_pos) = anim_interact.hover_pos() {
                                let (gx, gy) = to_game(mouse_pos);
                                self.project.salvage_placement.x = gx;
                                self.project.salvage_placement.y = gy;
                            }
                        }
                    }
                }
            });
        }
    }

}
