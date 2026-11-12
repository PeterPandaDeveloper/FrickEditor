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
use models::{AnimatronicArchetype, AnimatronicProject, CameraRouteNode, CountermeasureType, EnvironmentEntity, MazePropType};
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
    // Estado de Simulación de Salvamento (Room 18: SalvageResults)
    salvage_tension: f32,
    salvage_shocks_remaining: u8,
    salvage_audio_playing: bool,
    salvage_audio_timer: f32,
    salvage_shock_flash: f32,
    salvage_jumpscare_triggered: bool,
    salvage_prompt_step: u8,
    salvage_sheet_checks: [bool; 5],
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
            salvage_tension: 0.0,
            salvage_shocks_remaining: 3,
            salvage_audio_playing: false,
            salvage_audio_timer: 0.0,
            salvage_shock_flash: 0.0,
            salvage_jumpscare_triggered: false,
            salvage_prompt_step: 1,
            salvage_sheet_checks: [false; 5],
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
        self.sim_time += dt;
        if self.salvage_audio_playing {
            self.salvage_audio_timer -= dt;
            self.salvage_tension = (self.salvage_tension + dt * 11.5).min(100.0);
            if self.salvage_audio_timer <= 0.0 {
                self.salvage_audio_playing = false;
            }
        }
        if self.salvage_tension >= 100.0 && !self.salvage_jumpscare_triggered {
            self.salvage_jumpscare_triggered = true;
        }
        if self.salvage_shock_flash > 0.0 {
            self.salvage_shock_flash = (self.salvage_shock_flash - dt * 2.8).max(0.0);
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
                    RichText::new(format!("{}  6. Constructor 3D", ic::CUBE))
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
    // PESTAÑA 6: CONSTRUCTOR 3D (LABERINTOS, CONDUCTOS & MODO ENDLESS)
    // --------------------------------------------------------------
    fn ui_environment_3d_builder_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Selector de Modo Superior: Simulador Jugable vs Plano Técnico
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.salvage_tab_mode,
                SalvageTabMode::LiveSimulator,
                RichText::new(format!("{}  Simulador Jugable a 60 FPS (Room 18: Salvamento)", ic::GAME_CONTROLLER))
                    .strong()
                    .size(13.5)
                    .color(Color32::from_rgb(255, 185, 60)),
            );
            ui.add_space(10.0);
            ui.selectable_value(
                &mut self.salvage_tab_mode,
                SalvageTabMode::BlueprintEditor,
                RichText::new(format!("{}  Plano Técnico Cenital & Entidades (Coordenadas)", ic::COMPASS))
                    .size(13.0),
            );
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        if self.salvage_tab_mode == SalvageTabMode::LiveSimulator {
            let t = self.sim_time;
            let body_t = self.project.body_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
            let head_t = self.project.head_normal_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
            let scare_t = self.project.head_scare_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 320));
            let j0_t = self.project.jumpscare_frame_0.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 420));
            let j1_t = self.project.jumpscare_frame_1.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 420));
            let j2_t = self.project.jumpscare_frame_2.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 420));

            ui.columns(2, |cols| {
                // COLUMNA 1: CONTROLES DE SALVAMENTO & CHECKLIST
                cols[0].heading(format!("{}  Mesa de Salvamento (Room 18: SalvageResults)", ic::SHIELD));
                cols[0].label("Pon a prueba al animatrónico con los prompts de audio. Si se mueve o acumula tensión, aplica una descarga eléctrica antes de que ataque.");
                cols[0].add_space(8.0);

                // Barra de Tensión
                let tension_pct = (self.salvage_tension / 100.0).clamp(0.0, 1.0);
                let (tension_col, tension_status) = if self.salvage_tension < 35.0 {
                    (Color32::from_rgb(70, 215, 110), "FASE 1: Sujeto Inerte (Estable)")
                } else if self.salvage_tension < 75.0 {
                    (Color32::from_rgb(240, 185, 40), "FASE 2: Movimiento Detectado (Alerta)")
                } else {
                    (Color32::from_rgb(240, 65, 65), "FASE 3: HOSTILIDAD INMINENTE (Peligro)")
                };

                egui::Frame::group(cols[0].style())
                    .fill(Color32::from_rgb(22, 25, 32))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 80)))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(8.0)
                    .show(&mut cols[0], |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Nivel de Tensión:").strong());
                            ui.label(RichText::new(format!("{:.0}%", self.salvage_tension)).strong().color(tension_col));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(tension_status).size(11.0).color(tension_col));
                            });
                        });
                        ui.add_space(4.0);
                        let bar = egui::ProgressBar::new(tension_pct)
                            .fill(tension_col)
                            .animate(self.salvage_audio_playing);
                        ui.add(bar);
                    });

                cols[0].add_space(8.0);

                // Botones Interactivos: Grabadora y Electrochoque
                cols[0].horizontal(|ui| {
                    if self.salvage_audio_playing {
                        let audio_lbl = format!("{} Reproduciendo ({:.1}s)...", ic::SPEAKER_HIGH, self.salvage_audio_timer);
                        ui.add_enabled(false, egui::Button::new(RichText::new(audio_lbl).color(Color32::YELLOW)));
                    } else {
                        let btn_txt = format!("{} Reproducir Audio Prompt #{}", ic::PLAY, (self.salvage_prompt_step % 5) + 1);
                        let can_play = !self.salvage_jumpscare_triggered;
                        if ui.add_enabled(can_play, egui::Button::new(RichText::new(btn_txt).strong().color(Color32::WHITE)).fill(Color32::from_rgb(40, 110, 200))).clicked() {
                            self.salvage_audio_playing = true;
                            self.salvage_audio_timer = 4.0;
                            let snd = self.project.sound_knock_alert.clone();
                            if let Some(p) = snd {
                                self.play_audio(&p);
                            }
                        }
                    }

                    let taser_can_use = self.salvage_shocks_remaining > 0 && !self.salvage_jumpscare_triggered;
                    let taser_txt = format!("{} Taser ({} usos)", ic::LIGHTNING, self.salvage_shocks_remaining);
                    let taser_btn = egui::Button::new(RichText::new(taser_txt).strong().color(Color32::BLACK))
                        .fill(if taser_can_use { Color32::from_rgb(255, 200, 40) } else { Color32::GRAY });

                    if ui.add_enabled(taser_can_use, taser_btn).clicked() {
                        self.salvage_shocks_remaining = self.salvage_shocks_remaining.saturating_sub(1);
                        self.salvage_shock_flash = 1.0;
                        self.salvage_tension = (self.salvage_tension - 45.0).max(0.0);
                        let snd = self.project.sound_defeat.clone();
                        if let Some(p) = snd {
                            self.play_audio(&p);
                        }
                    }
                });

                cols[0].add_space(8.0);
                cols[0].separator();
                cols[0].add_space(4.0);

                // Hoja de Documentación (Checklist)
                cols[0].label(RichText::new(format!("{} Hoja de Documentación de Salvamento:", ic::CLIPBOARD_TEXT)).strong());
                cols[0].add_space(4.0);

                let check_labels = [
                    "Prompt 1: Verificación de respuesta motriz",
                    "Prompt 2: Prueba de frecuencia y modulación de audio",
                    "Prompt 3: Estimulación de reconocimiento óptico",
                    "Prompt 4: Test de sincronización de servomotores",
                    "Prompt 5: Medición de hostilidad agresiva latente",
                ];

                for i in 0..5 {
                    cols[0].horizontal(|ui| {
                        let is_current = (self.salvage_prompt_step as usize) == i;
                        if ui.checkbox(&mut self.salvage_sheet_checks[i], check_labels[i]).changed() {
                            if self.salvage_sheet_checks[i] && is_current {
                                self.salvage_prompt_step = (i + 1) as u8;
                            }
                        }
                    });
                }

                cols[0].add_space(6.0);
                let completed_count = self.salvage_sheet_checks.iter().filter(|c| **c).count();
                if completed_count == 5 && !self.salvage_jumpscare_triggered {
                    egui::Frame::group(cols[0].style())
                        .fill(Color32::from_rgb(18, 45, 25))
                        .stroke(Stroke::new(1.5_f32, Color32::from_rgb(70, 215, 110)))
                        .inner_margin(8.0)
                        .show(&mut cols[0], |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(ic::CHECK_CIRCLE).size(20.0).color(Color32::from_rgb(80, 230, 120)));
                                ui.label(RichText::new("¡SALVAMENTO COMPLETADO CON ÉXITO!\nRecompensa de la noche: $500 añadidos a fondos.").strong().color(Color32::WHITE));
                            });
                        });
                }

                cols[0].add_space(8.0);
                if cols[0].button(format!("{} Reiniciar Prueba de Salvamento", ic::ARROWS_CLOCKWISE)).clicked() {
                    self.salvage_tension = 0.0;
                    self.salvage_shocks_remaining = 3;
                    self.salvage_audio_playing = false;
                    self.salvage_audio_timer = 0.0;
                    self.salvage_shock_flash = 0.0;
                    self.salvage_jumpscare_triggered = false;
                    self.salvage_prompt_step = 0;
                    self.salvage_sheet_checks = [false; 5];
                    self.stop_audio();
                }

                // COLUMNA 2: LIENZO JUGABLE A 60 FPS
            cols[1].heading(format!("{}  Sala de Salvamento (Cámara en Primera Persona)", ic::MONITOR_PLAY));
            cols[1].label("Perspectiva del guardia frente a la mesa de inspección:");
            cols[1].add_space(8.0);

            let (rect, resp) = cols[1].allocate_exact_size(Vec2::new(540.0, 360.0), Sense::click());
            let painter = cols[1].painter_at(rect);

            // 1. Fondo de la sala oscura
            painter.rect_filled(rect, 6.0, Color32::from_rgb(10, 12, 16));

            // 2. Luz de cono cenital (Spotlight central parpadeante)
            let flicker = if self.salvage_audio_playing { (t * 25.0).sin() * 8.0 } else { (t * 3.0).sin() * 3.0 };
            let spot_center = Pos2::new(rect.center().x, rect.min.y + 115.0);
            painter.circle_filled(spot_center, 120.0 + flicker, Color32::from_rgba_unmultiplied(240, 220, 170, 18));
            painter.circle_filled(spot_center, 75.0 + (flicker * 0.5), Color32::from_rgba_unmultiplied(255, 235, 190, 24));

            // 3. Silla de salvamento detrás del sujeto
            let chair_rect = Rect::from_center_size(Pos2::new(spot_center.x, spot_center.y + 40.0), Vec2::new(160.0, 150.0));
            painter.rect_filled(chair_rect, 4.0, Color32::from_rgb(25, 22, 20));
            painter.rect_stroke(chair_rect, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(50, 42, 38)));

            // 4. Renderizado del Animatrónico en la Silla
            if self.salvage_jumpscare_triggered {
                // JUMPSCARE DE SALVAMENTO
                let cycle = (t % 1.2) / 1.2;
                let zoom = 0.70 + (cycle.powf(0.40) * 0.75);
                let sz = Vec2::new(340.0 * zoom, 340.0 * zoom);
                let j_shake_x = (t * 80.0).sin() * 8.0;
                let j_pos = Pos2::new(rect.center().x + j_shake_x, rect.center().y + (10.0 * zoom));

                if let Some(tex) = &j0_t {
                    painter.image(tex.id(), Rect::from_center_size(j_pos, sz), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                let head_choice = if ((t * 18.0) as i32) % 2 == 0 { &j1_t } else { &j2_t };
                if let Some(tex) = head_choice {
                    let jaw_jitter = (t * 40.0).sin() * 4.0 * zoom;
                    painter.image(tex.id(), Rect::from_center_size(Pos2::new(j_pos.x, j_pos.y + jaw_jitter), sz), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }

                // Marco rojo de muerte
                painter.rect_stroke(rect, 6.0, Stroke::new(6.0_f32, Color32::from_rgb(240, 40, 40)));
                painter.text(
                    Pos2::new(rect.center().x, rect.min.y + 30.0),
                    egui::Align2::CENTER_CENTER,
                    "¡HAS SIDO ASESINADO! — EL SUJETO ATACÓ",
                    egui::FontId::proportional(16.0),
                    Color32::from_rgb(255, 80, 80),
                );
            } else {
                // FASES VIVAS DEL ANIMATRÓNICO
                let anim_center = Pos2::new(rect.center().x, rect.min.y + 130.0);

                if self.salvage_tension < 35.0 {
                    // Fase 1: Sujeto Inerte / Cabeza Baja
                    let breathe = (t * 2.0).sin() * 2.5;
                    if let Some(tex) = &body_t {
                        let b_rect = Rect::from_center_size(Pos2::new(anim_center.x, anim_center.y + 50.0 + breathe), Vec2::new(170.0, 125.0));
                        painter.image(tex.id(), b_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    if let Some(tex) = &head_t {
                        let h_rect = Rect::from_center_size(Pos2::new(anim_center.x, anim_center.y - 12.0 + (breathe * 0.7)), Vec2::new(145.0, 125.0));
                        painter.image(tex.id(), h_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                } else if self.salvage_tension < 75.0 {
                    // Fase 2: Alerta / Movimiento y Tics
                    let twitch = if (t * 8.0).sin() > 0.6 { (t * 45.0).cos() * 3.5 } else { 0.0 };
                    if let Some(tex) = &body_t {
                        let b_rect = Rect::from_center_size(Pos2::new(anim_center.x, anim_center.y + 45.0), Vec2::new(170.0, 125.0));
                        painter.image(tex.id(), b_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    if let Some(tex) = &head_t {
                        let h_rect = Rect::from_center_size(Pos2::new(anim_center.x + twitch, anim_center.y - 18.0), Vec2::new(148.0, 128.0));
                        painter.image(tex.id(), h_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                } else {
                    // Fase 3: Hostilidad Inminente / Temblores Fuertes & Mirada Fija
                    let tremor_x = (t * 55.0).sin() * 4.0;
                    let tremor_y = (t * 65.0).cos() * 3.0;
                    if let Some(tex) = &body_t {
                        let b_rect = Rect::from_center_size(Pos2::new(anim_center.x + tremor_x, anim_center.y + 40.0), Vec2::new(175.0, 130.0));
                        painter.image(tex.id(), b_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    let host_head = scare_t.as_ref().or(j1_t.as_ref()).or(head_t.as_ref());
                    if let Some(tex) = host_head {
                        let h_rect = Rect::from_center_size(Pos2::new(anim_center.x + tremor_x, anim_center.y - 25.0 + tremor_y), Vec2::new(155.0, 135.0));
                        painter.image(tex.id(), h_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }

                    // Ojos brillantes de agresión
                    painter.circle_filled(Pos2::new(anim_center.x - 18.0 + tremor_x, anim_center.y - 30.0 + tremor_y), 3.5, Color32::from_rgb(255, 60, 40));
                    painter.circle_filled(Pos2::new(anim_center.x + 18.0 + tremor_x, anim_center.y - 30.0 + tremor_y), 3.5, Color32::from_rgb(255, 60, 40));
                }

                // 5. Mesa de trabajo en primer plano (Foreground)
                let table_rect = Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - 75.0), rect.max);
                painter.rect_filled(table_rect, 0.0, Color32::from_rgb(32, 22, 16));
                painter.line_segment([table_rect.left_top(), table_rect.right_top()], Stroke::new(2.5_f32, Color32::from_rgb(60, 42, 32)));

                // Grabadora en la mesa (Izquierda)
                let cassette_box = Rect::from_center_size(Pos2::new(rect.min.x + 95.0, rect.max.y - 38.0), Vec2::new(90.0, 48.0));
                painter.rect_filled(cassette_box, 3.0, Color32::from_rgb(45, 50, 58));
                painter.rect_stroke(cassette_box, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(80, 85, 95)));
                painter.text(cassette_box.center(), egui::Align2::CENTER_CENTER, format!("{} CASSETTE", ic::SPEAKER_HIGH), egui::FontId::proportional(10.5), Color32::WHITE);

                // Hoja de documentación en la mesa (Centro)
                let paper_box = Rect::from_center_size(Pos2::new(rect.center().x, rect.max.y - 35.0), Vec2::new(110.0, 52.0));
                painter.rect_filled(paper_box, 2.0, Color32::from_rgb(220, 215, 195));
                painter.rect_stroke(paper_box, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(160, 150, 130)));
                painter.text(paper_box.center(), egui::Align2::CENTER_CENTER, "DOCUMENTS", egui::FontId::proportional(11.0), Color32::BLACK);

                // Taser en la mesa (Derecha)
                let taser_box = Rect::from_center_size(Pos2::new(rect.max.x - 95.0, rect.max.y - 38.0), Vec2::new(85.0, 45.0));
                let taser_col = if self.salvage_shocks_remaining > 0 { Color32::from_rgb(210, 160, 30) } else { Color32::from_rgb(55, 58, 65) };
                painter.rect_filled(taser_box, 3.0, taser_col);
                painter.rect_stroke(taser_box, 3.0, Stroke::new(1.0_f32, Color32::WHITE));
                painter.text(taser_box.center(), egui::Align2::CENTER_CENTER, format!("{} SHOCK", ic::LIGHTNING), egui::FontId::proportional(11.0), Color32::BLACK);
            }

            // 6. Efecto de Flash de Electrochoque
            if self.salvage_shock_flash > 0.0 {
                let alpha = (self.salvage_shock_flash * 220.0).min(255.0) as u8;
                painter.rect_filled(rect, 6.0, Color32::from_rgba_unmultiplied(210, 240, 255, alpha));

                // Líneas de relámpago
                let cx = rect.center().x;
                let cy = rect.center().y;
                painter.line_segment([Pos2::new(cx - 60.0, cy - 80.0), Pos2::new(cx, cy)], Stroke::new(3.0_f32, Color32::WHITE));
                painter.line_segment([Pos2::new(cx, cy), Pos2::new(cx + 50.0, cy + 60.0)], Stroke::new(3.0_f32, Color32::WHITE));
            }

            // Clic en la pantalla para interactuar
            if resp.clicked() && !self.salvage_jumpscare_triggered {
                self.salvage_tension = (self.salvage_tension + 8.0).min(100.0);
            }
            });
        } else {
            // ==============================================================
            // MODO PLANO TÉCNICO CENITAL & CATÁLOGO DE ENTIDADES 3D
            // ==============================================================
            ui.columns(2, |cols| {
                // Columna Izquierda: Paleta de Objetos (Drag & Drop)
                cols[0].heading(format!("{}  Catálogo de Entidades 3D & Props", ic::TOOLBOX));
                cols[0].label("Haz clic para añadir objetos interactivos al laberinto (basado en Room 12-16):");
                cols[0].add_space(8.0);

                // Selector de Sala Destino
                cols[0].horizontal(|ui| {
                    ui.label(RichText::new("Sala Destino:").strong());
                    let rooms = [
                        "MinigameMaze_1",
                        "MinigameMaze_2",
                        "MinigameMaze_3",
                        "MinigameMaze_4",
                        "MinigameMaze_5",
                        "Room 18: SalvageResults (Sala de Salvamento)",
                        "Room 8: SalvageEnding (Final de Salvamento)",
                        "MinigameMaze_Vents",
                        "Fredbear_3DMaze",
                    ];
                    egui::ComboBox::from_label("")
                        .selected_text(&self.project.advanced.target_maze_room)
                        .show_ui(ui, |ui| {
                            for rm in rooms {
                                ui.selectable_value(&mut self.project.advanced.target_maze_room, rm.to_string(), rm);
                            }
                        });
                });

                cols[0].add_space(8.0);

                egui::ScrollArea::vertical().id_salt("prop_palette_scroll").max_height(280.0).show(&mut cols[0], |ui| {
                    let props = [
                        (MazePropType::AnimatronicSpawn, "Punto de Interacción / Susto (Animatronic)", ic::GHOST),
                        (MazePropType::Hatchet, "Hacha (Interactuable en Suelo)", ic::SWORD),
                        (MazePropType::GiftBox, "Caja de Regalo (Prop)", ic::GIFT),
                        (MazePropType::Poster, "Póster de Pared", ic::IMAGE),
                        (MazePropType::ArcadeCabinet, "Máquina Arcade", ic::GAME_CONTROLLER),
                        (MazePropType::EscapeDoor, "Puerta de Escape / Salida", ic::DOOR_OPEN),
                        (MazePropType::SalvageChair, "Silla de Salvamento / Inspección (Room 18)", ic::SHIELD),
                        (MazePropType::SalvageCassette, "Grabadora de Cassette (Audio Prompt)", ic::SPEAKER_HIGH),
                        (MazePropType::SalvageShock, "Taser de Electrochoque (3 usos)", ic::LIGHTNING),
                    ];

                    for (prop_type, desc, icon) in props.iter() {
                        egui::Frame::group(ui.style())
                            .fill(Color32::from_rgb(28, 31, 38))
                            .stroke(Stroke::new(1.0_f32, Color32::from_gray(100)))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(*icon).size(18.0).color(Color32::from_rgb(200, 150, 50)));
                                    ui.label(RichText::new(*desc).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button(format!("{} Añadir al Mapa", ic::PLUS)).clicked() {
                                            let offset = (self.project.advanced.maze_entities.len() * 24) as i32;
                                            self.project.advanced.maze_entities.push(EnvironmentEntity {
                                                prop_type: prop_type.clone(),
                                                x: 320 + (offset % 120),
                                                y: 180 + (offset % 80),
                                                scale_x: 1.0,
                                                scale_y: 1.0,
                                            });
                                        }
                                    });
                                });
                            });
                        ui.add_space(6.0);
                    }
                });

                cols[0].add_space(6.0);
                if self.project.advanced.target_maze_room.contains("Salvage") {
                    if cols[0].button(RichText::new(format!("{} Auto-configurar Setup Room 18", ic::LIGHTNING)).strong().color(Color32::from_rgb(255, 200, 80))).clicked() {
                        self.project.advanced.maze_entities.clear();
                        self.project.advanced.maze_entities.push(EnvironmentEntity {
                            prop_type: MazePropType::SalvageChair,
                            x: 320,
                            y: 110,
                            scale_x: 1.0,
                            scale_y: 1.0,
                        });
                        self.project.advanced.maze_entities.push(EnvironmentEntity {
                            prop_type: MazePropType::AnimatronicSpawn,
                            x: 320,
                            y: 80,
                            scale_x: 1.0,
                            scale_y: 1.0,
                        });
                        self.project.advanced.maze_entities.push(EnvironmentEntity {
                            prop_type: MazePropType::SalvageCassette,
                            x: 160,
                            y: 280,
                            scale_x: 1.0,
                            scale_y: 1.0,
                        });
                        self.project.advanced.maze_entities.push(EnvironmentEntity {
                            prop_type: MazePropType::SalvageShock,
                            x: 480,
                            y: 280,
                            scale_x: 1.0,
                            scale_y: 1.0,
                        });
                    }
                    cols[0].add_space(4.0);
                }
                cols[0].horizontal(|ui| {
                    ui.label(RichText::new(format!("Entidades colocadas: {}", self.project.advanced.maze_entities.len())).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{} Limpiar Todo", ic::TRASH)).clicked() {
                            self.project.advanced.maze_entities.clear();
                        }
                    });
                });

                // Columna Derecha: Lienzo 2D Cenital Interactivo (Cálculo de Coordenadas In-Game)
                cols[1].heading(format!("{}  Plano Cenital del Entorno", ic::MAP_TRIFOLD));
                cols[1].label("Arrastra las entidades con el ratón. Clic derecho sobre una entidad para eliminarla:");
                cols[1].add_space(8.0);

                let (rect, _resp) = cols[1].allocate_exact_size(Vec2::new(540.0, 360.0), Sense::click_and_drag());
                let painter = cols[1].painter_at(rect);

                // Fondo simulando el plano técnico (Grid)
                painter.rect_filled(rect, 4.0, Color32::from_rgb(15, 20, 25));
                for i in 0..17 {
                    let offset_x = i as f32 * 32.0;
                    painter.line_segment(
                        [Pos2::new(rect.min.x + offset_x, rect.min.y), Pos2::new(rect.min.x + offset_x, rect.max.y)],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 12)),
                    );
                }
                for i in 0..12 {
                    let offset_y = i as f32 * 32.0;
                    painter.line_segment(
                        [Pos2::new(rect.min.x, rect.min.y + offset_y), Pos2::new(rect.max.x, rect.min.y + offset_y)],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 12)),
                    );
                }
                painter.rect_stroke(rect, 4.0, Stroke::new(2.0_f32, Color32::GRAY));

                let is_salvage = self.project.advanced.target_maze_room.contains("Salvage");
                if is_salvage {
                    // Haz de luz cenital (Spotlight de la sala de salvamento)
                    painter.circle_filled(Pos2::new(rect.center().x, rect.min.y + 110.0), 105.0, Color32::from_rgba_unmultiplied(255, 235, 180, 20));
                    painter.circle_stroke(Pos2::new(rect.center().x, rect.min.y + 110.0), 105.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 235, 180, 45)));

                    // Zona del Sujeto / Silla de Inspección
                    let chair_box = Rect::from_center_size(Pos2::new(rect.center().x, rect.min.y + 110.0), Vec2::new(140.0, 95.0));
                    painter.rect_filled(chair_box, 4.0, Color32::from_rgba_unmultiplied(70, 35, 20, 140));
                    painter.rect_stroke(chair_box, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(200, 130, 70)));
                    painter.text(Pos2::new(chair_box.center().x, chair_box.min.y + 16.0), egui::Align2::CENTER_CENTER, "ZONA SUJETO DE PRUEBA", egui::FontId::proportional(11.0), Color32::from_rgb(255, 180, 100));

                    // Mesa de Documentación / Taser / Cassette (Primer Plano)
                    let table_rect = Rect::from_min_max(Pos2::new(rect.min.x + 30.0, rect.min.y + 230.0), Pos2::new(rect.max.x - 30.0, rect.max.y - 15.0));
                    painter.rect_filled(table_rect, 6.0, Color32::from_rgba_unmultiplied(40, 25, 15, 220));
                    painter.rect_stroke(table_rect, 6.0, Stroke::new(2.0_f32, Color32::from_rgb(110, 75, 45)));
                    painter.text(table_rect.center(), egui::Align2::CENTER_CENTER, "MESA DE TRABAJO (Grabadora / Taser / Hoja de Salvamento)", egui::FontId::proportional(11.0), Color32::from_rgb(190, 160, 130));

                    // Indicador HUD de Tensión & Shocks
                    let hud_rect = Rect::from_min_max(Pos2::new(rect.min.x + 10.0, rect.min.y + 8.0), Pos2::new(rect.max.x - 10.0, rect.min.y + 30.0));
                    painter.rect_filled(hud_rect, 3.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));
                    painter.rect_stroke(hud_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(100, 100, 100)));
                    painter.text(
                        hud_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "ROOM 18: SALVAGE RESULTS — Tensión: 0% / 100% | Shocks: 3 | Valor: $500",
                        egui::FontId::proportional(11.5),
                        Color32::from_rgb(255, 215, 0),
                    );
                }

                let mut to_delete = None;
                let scale_canvas_x = 540.0 / 640.0;

                for (idx, entity) in self.project.advanced.maze_entities.iter_mut().enumerate() {
                    let ex = rect.min.x + (entity.x as f32 * scale_canvas_x);
                    let ey = rect.min.y + entity.y as f32;
                    let entity_rect = Rect::from_center_size(Pos2::new(ex, ey), Vec2::new(26.0, 26.0));

                    let icon_str = match entity.prop_type {
                        MazePropType::AnimatronicSpawn => ic::GHOST,
                        MazePropType::Hatchet => ic::SWORD,
                        MazePropType::GiftBox => ic::GIFT,
                        MazePropType::Poster => ic::IMAGE,
                        MazePropType::ArcadeCabinet => ic::GAME_CONTROLLER,
                        MazePropType::EscapeDoor => ic::DOOR_OPEN,
                        MazePropType::SalvageChair => ic::SHIELD,
                        MazePropType::SalvageCassette => ic::SPEAKER_HIGH,
                        MazePropType::SalvageShock => ic::LIGHTNING,
                    };

                    let color = match entity.prop_type {
                        MazePropType::AnimatronicSpawn => Color32::RED,
                        MazePropType::Hatchet => Color32::LIGHT_GRAY,
                        MazePropType::GiftBox => Color32::from_rgb(230, 180, 40),
                        MazePropType::EscapeDoor => Color32::from_rgb(60, 200, 80),
                        MazePropType::SalvageChair => Color32::from_rgb(180, 100, 50),
                        MazePropType::SalvageCassette => Color32::from_rgb(70, 150, 230),
                        MazePropType::SalvageShock => Color32::from_rgb(240, 200, 40),
                        _ => Color32::from_rgb(100, 180, 255),
                    };

                    painter.rect_filled(entity_rect, 4.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 190));
                    painter.rect_stroke(entity_rect, 4.0, Stroke::new(1.5_f32, Color32::WHITE));
                    painter.text(entity_rect.center(), egui::Align2::CENTER_CENTER, icon_str, egui::FontId::proportional(14.0), Color32::WHITE);

                    // Lógica de Drag & Drop
                    let id = egui::Id::new(format!("entity_drag_{}", idx));
                    let interact = cols[1].interact(entity_rect, id, Sense::drag());
                    if interact.dragged() {
                        let dx = (interact.drag_delta().x / scale_canvas_x) as i32;
                        let dy = interact.drag_delta().y as i32;
                        entity.x = (entity.x + dx).clamp(0, 640);
                        entity.y = (entity.y + dy).clamp(0, 360);
                    }
                    if interact.clicked_by(egui::PointerButton::Secondary) {
                        to_delete = Some(idx);
                    }

                    // Tooltip de coordenadas
                    if interact.hovered() || interact.dragged() {
                        painter.text(
                            Pos2::new(entity_rect.center().x, entity_rect.max.y + 10.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{}: ({}, {})", entity.prop_type.gml_object_name(), entity.x, entity.y),
                            egui::FontId::proportional(10.5),
                            Color32::YELLOW,
                        );
                    }
                }

                if let Some(idx) = to_delete {
                    self.project.advanced.maze_entities.remove(idx);
                }

                cols[1].add_space(6.0);
                cols[1].label(
                    RichText::new("Usa Clic Izquierdo para arrastrar entidades. Clic Derecho sobre una entidad para eliminarla.")
                        .size(12.0)
                        .color(Color32::from_gray(150)),
                );
            });
        }
    }
}
