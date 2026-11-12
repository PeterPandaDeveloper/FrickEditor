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

                if let Some(tex) = &j0_t {
                    let r = Rect::from_center_size(Pos2::new(center.x + shake_x, center.y + 30.0 + shake_y), sz);
                    painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                let head_choice = if ((t * 15.0) as i32) % 2 == 0 { &j1_t } else { &j2_t };
                if let Some(tex) = head_choice {
                    let jaw_y = (t * 35.0).sin() * 6.0 * zoom;
                    let r = Rect::from_center_size(Pos2::new(center.x + shake_x, center.y - 10.0 + shake_y + jaw_y), sz);
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
                    if let Some(tex) = &j0_t {
                        let r = Rect::from_center_size(Pos2::new(center.x, center.y + 20.0), sz);
                        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    let head_choice = if ((t * 15.0) as i32) % 2 == 0 { &j1_t } else { &j2_t };
                    if let Some(tex) = head_choice {
                        let jaw_y = (t * 35.0).sin() * 6.0 * zoom;
                        let r = Rect::from_center_size(Pos2::new(center.x, center.y - 10.0 + jaw_y), sz);
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
            // Columna Izquierda: Selector Visual de Orden en la Página de Mods (32 Slots = 8x4)
            cols[0].heading(format!("{}  Posición del Tile en Custom Night", ic::GRID_FOUR));
            cols[0].label("Haz clic en cualquier casilla para elegir en qué orden aparece en la Página 2 (8x4):");
            cols[0].add_space(6.0);

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

            // Predefinidos de rutas
            cols[0].horizontal(|ui| {
                if ui.button(format!("{} Pasillo Oeste (1->2->6->9)", ic::FLOW_ARROW)).clicked() {
                    self.project.advanced.camera_route = vec![
                        CameraRouteNode { camera_id: 1, camera_name: "CAM 01: Show Stage".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 2, camera_name: "CAM 02: Dining Area".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 6, camera_name: "CAM 06: West Hallway".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 9, camera_name: "CAM 09: Office Doorway".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                    ];
                }
                if ui.button(format!("{} Pasillo Este (1->2->4->5->9)", ic::FLOW_ARROW)).clicked() {
                    self.project.advanced.camera_route = vec![
                        CameraRouteNode { camera_id: 1, camera_name: "CAM 01: Show Stage".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 2, camera_name: "CAM 02: Dining Area".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 4, camera_name: "CAM 04: East Hall".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 5, camera_name: "CAM 05: Kitchen Area".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
                        CameraRouteNode { camera_id: 9, camera_name: "CAM 09: Office Doorway".to_string(), sprite_path: None, transition_time_modifier: 1.0 },
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
                "// Array de movimiento en cámaras\n\
                 global.{id}_CameraRoute = [{cams}];\n\
                 global.{id}_RouteStep = 0;\n\
                 global.{id}_NextCamTimer = round(180 * {speed:.1});",
                id = self.project.internal_id,
                cams = cam_ids.join(", "),
                speed = self.project.advanced.camera_route.first().map(|n| n.transition_time_modifier).unwrap_or(1.0)
            );
            cols[0].add(egui::TextEdit::multiline(&mut preview_code).desired_rows(4).desired_width(f32::INFINITY));

            // Columna 2: Plano Topológico Interactivo de Cámaras
            cols[1].heading(format!("{}  Plano Topológico del Restaurante", ic::BUILDING));
            cols[1].label("Haz clic en cualquier cámara para añadirla o retirarla de la ruta:");
            cols[1].add_space(8.0);

            let (rect, _resp) = cols[1].allocate_exact_size(Vec2::new(540.0, 360.0), Sense::click());
            let painter = cols[1].painter_at(rect);

            // Fondo del mapa
            painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 22, 28));
            painter.rect_stroke(rect, 6.0, Stroke::new(1.5_f32, Color32::from_rgb(45, 55, 70)));

            // Habitaciones esquemáticas
            let draw_room = |p: &egui::Painter, r: Rect, title: &str| {
                p.rect_filled(r, 4.0, Color32::from_rgb(24, 30, 40));
                p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(55, 68, 88)));
                p.text(Pos2::new(r.center().x, r.min.y + 10.0), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(11.0), Color32::from_gray(160));
            };

            let ox = rect.min.x;
            let oy = rect.min.y;

            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 190.0, oy + 20.0), Vec2::new(160.0, 70.0)), "SHOW STAGE");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 170.0, oy + 105.0), Vec2::new(200.0, 110.0)), "DINING AREA");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 20.0, oy + 40.0), Vec2::new(130.0, 70.0)), "BACKSTAGE");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 20.0, oy + 125.0), Vec2::new(130.0, 90.0)), "PIRATE COVE");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 390.0, oy + 105.0), Vec2::new(130.0, 90.0)), "EAST HALL");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 390.0, oy + 210.0), Vec2::new(130.0, 80.0)), "KITCHEN");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 20.0, oy + 230.0), Vec2::new(130.0, 70.0)), "SUPPLY CLOSET");
            draw_room(&painter, Rect::from_min_size(Pos2::new(ox + 200.0, oy + 270.0), Vec2::new(140.0, 75.0)), "THE OFFICE");

            // Definición de las 9 cámaras
            let cameras = [
                (1u8, "CAM 01", Pos2::new(ox + 270.0, oy + 65.0), "CAM 01: Show Stage"),
                (2u8, "CAM 02", Pos2::new(ox + 270.0, oy + 160.0), "CAM 02: Dining Area"),
                (3u8, "CAM 03", Pos2::new(ox + 85.0, oy + 175.0), "CAM 03: Pirate Cove"),
                (4u8, "CAM 04", Pos2::new(ox + 455.0, oy + 150.0), "CAM 04: East Hall"),
                (5u8, "CAM 05", Pos2::new(ox + 455.0, oy + 250.0), "CAM 05: Kitchen Area"),
                (6u8, "CAM 06", Pos2::new(ox + 210.0, oy + 235.0), "CAM 06: West Hallway"),
                (7u8, "CAM 07", Pos2::new(ox + 85.0, oy + 265.0), "CAM 07: Supply Closet"),
                (8u8, "CAM 08", Pos2::new(ox + 85.0, oy + 75.0), "CAM 08: Backstage"),
                (9u8, "CAM 09", Pos2::new(ox + 270.0, oy + 245.0), "CAM 09: Office Doorway"),
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
                    painter.line_segment([c.2, Pos2::new(ox + 270.0, oy + 305.0)], Stroke::new(2.0_f32, Color32::from_rgb(230, 60, 60)));
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
    fn ui_environment_3d_builder_tab(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
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
                    "MinigameMaze_Vents",
                    "MinigameMaze_Endless",
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
                };

                let color = match entity.prop_type {
                    MazePropType::AnimatronicSpawn => Color32::RED,
                    MazePropType::Hatchet => Color32::LIGHT_GRAY,
                    MazePropType::GiftBox => Color32::from_rgb(230, 180, 40),
                    MazePropType::EscapeDoor => Color32::from_rgb(60, 200, 80),
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
