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
use models::{AnimatronicArchetype, AnimatronicProject, CameraRouteNode, CountermeasureType, MazeRoomData, MazeRoomInstance, SalvageLocationIndex};
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
    maze_player_x: f32,
    maze_player_y: f32,
    maze_player_yaw: f32,
    // Usabilidad, Ergonomía y UI
    toast_message: Option<(String, bool, f32)>,
    noclip_mode: bool,
    move_speed_multiplier: f32,
    radar_display_mode: usize,
    show_crosshair: bool,
    show_audit_window: bool,
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
            maze_player_x: 960.0,
            maze_player_y: 224.0,
            maze_player_yaw: 180.0,
            toast_message: None,
            noclip_mode: false,
            move_speed_multiplier: 1.0,
            radar_display_mode: 0,
            show_crosshair: true,
            show_audit_window: false,
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

        // Actualizar temporizador de Toast flotante
        if let Some((_, _, ref mut time)) = self.toast_message {
            *time -= dt;
            if *time <= 0.0 {
                self.toast_message = None;
            }
        }

        // Atajos de Teclado Globales (Ergonomía & Comodidad)
        ctx.input(|i| {
            // Ctrl+S: Guardar Proyecto Rápido
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                let save_path = self.project.workspace_dir.join(format!("{}_project.json", self.project.internal_id.to_lowercase()));
                if let Ok(json) = serde_json::to_string_pretty(&self.project) {
                    if std::fs::write(&save_path, json).is_ok() {
                        self.show_toast(format!("Proyecto guardado en {}", save_path.file_name().unwrap_or_default().to_string_lossy()), false);
                    }
                }
            }
            // F5: PLAY (Inyectar y Jugar)
            if i.key_pressed(egui::Key::F5) {
                self.handle_one_click_play();
            }
            // Alt + 0..6: Cambiar de Pestaña Rápido
            if i.modifiers.alt {
                if i.key_pressed(egui::Key::Num0) { self.active_tab = ActiveTab::ModManager; }
                if i.key_pressed(egui::Key::Num1) { self.active_tab = ActiveTab::Gallery; }
                if i.key_pressed(egui::Key::Num2) { self.active_tab = ActiveTab::SpritesAndGuide; }
                if i.key_pressed(egui::Key::Num3) { self.active_tab = ActiveTab::LiveAnimationPreview; }
                if i.key_pressed(egui::Key::Num4) { self.active_tab = ActiveTab::GridAndMechanics; }
                if i.key_pressed(egui::Key::Num5) { self.active_tab = ActiveTab::CameraMapRouter; }
                if i.key_pressed(egui::Key::Num6) { self.active_tab = ActiveTab::Environment3DBuilder; }
            }
        });

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

                ui.add_space(4.0);
                let utmt_header_btn = egui::Button::new(
                    RichText::new(format!("{} UTMT", ic::CODE))
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(220, 235, 255)),
                )
                .fill(Color32::from_rgb(45, 65, 95))
                .min_size(Vec2::new(78.0, 26.0));

                if ui.add(utmt_header_btn).on_hover_text("Abrir UndertaleModTool directamente cargando data.win").clicked() {
                    match GameLauncher::open_undertale_mod_tool(&self.project.utmt_scripts_dir, &self.project.game_dir) {
                        Ok(msg) => {
                            self.status_is_error = false;
                            self.status_message = msg;
                            self.show_toast(format!("{} UTMT iniciado con data.win", ic::CHECK_CIRCLE), false);
                        }
                        Err(err) => {
                            self.status_is_error = true;
                            self.status_message = err.clone();
                            self.show_toast(format!("{} {}", ic::WARNING_CIRCLE, err), true);
                        }
                    }
                }

                ui.add_space(4.0);
                let audit_header_btn = egui::Button::new(
                    RichText::new(format!("{} Auditoría", ic::FILE_TEXT))
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(220, 245, 230)),
                )
                .fill(Color32::from_rgb(35, 75, 55))
                .min_size(Vec2::new(98.0, 26.0));

                if ui.add(audit_header_btn).on_hover_text("Ver qué se modificó en el juego, explicación de Phantoms/Puppet y auditoría de inyecciones").clicked() {
                    self.show_audit_window = true;
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

        // ==========================================================
        // NOTIFICACIÓN TOAST FLOTANTE EN ESQUINA INFERIOR DERECHA
        // ==========================================================
        if let Some((msg, is_err, time)) = &self.toast_message {
            let alpha = (*time / 0.5).min(1.0);
            let bg = if *is_err {
                Color32::from_rgba_unmultiplied(65, 18, 18, (235.0 * alpha) as u8)
            } else {
                Color32::from_rgba_unmultiplied(18, 48, 28, (235.0 * alpha) as u8)
            };
            let border = if *is_err {
                Color32::from_rgba_unmultiplied(240, 70, 70, (250.0 * alpha) as u8)
            } else {
                Color32::from_rgba_unmultiplied(60, 225, 110, (250.0 * alpha) as u8)
            };
            let icon = if *is_err { ic::WARNING_CIRCLE } else { ic::CHECK_CIRCLE };

            egui::Area::new(egui::Id::new("floating_toast_notification_area"))
                .fixed_pos(Pos2::new(ctx.screen_rect().max.x - 380.0, ctx.screen_rect().max.y - 85.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(bg)
                        .stroke(Stroke::new(1.5_f32, border))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(icon).size(18.0).color(border));
                                ui.label(RichText::new(msg).size(12.5).color(Color32::WHITE).strong());
                            });
                        });
                });
        }

        // ==========================================================
        // VENTANA MODAL FLOTANTE DE AUDITORÍA Y NOTA TÉCNICA
        // ==========================================================
        if self.show_audit_window {
            egui::Window::new(RichText::new(format!("{}  Auditoría Técnica de Modificaciones (data.win)", ic::FILE_TEXT)).strong().size(14.0))
                .default_width(720.0)
                .default_height(520.0)
                .resizable(true)
                .open(&mut self.show_audit_window)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new(format!("{}  Abrir en Notepad", ic::ARROW_SQUARE_OUT)).color(Color32::from_rgb(100, 200, 255))).clicked() {
                            let _ = GameLauncher::open_audit_note(&self.project.workspace_dir);
                        }
                        if ui.button(RichText::new(format!("{}  Abrir UndertaleModTool", ic::CODE)).color(Color32::from_rgb(255, 180, 80))).clicked() {
                            let _ = GameLauncher::open_undertale_mod_tool(&self.project.utmt_scripts_dir, &self.project.game_dir);
                        }
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let audit_path = self.project.workspace_dir.join("AUDITORIA_MODIFICACIONES_GAME.md");
                        let content = std::fs::read_to_string(&audit_path).unwrap_or_else(|_| {
                            let alt = PathBuf::from("FrickEditor_GitHub_Repository/docs/AUDITORIA_MODIFICACIONES_GAME.md");
                            std::fs::read_to_string(&alt).unwrap_or_else(|_| "No se pudo cargar la nota de auditoría.".to_string())
                        });
                        ui.label(RichText::new(content).monospace().size(12.0));
                    });
                });
        }
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

        // 5. INTEGRACIÓN DIRECTA CON UNDERTALE MOD TOOL (UTMT)
        ui.add_space(12.0);
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(22, 28, 38))
            .stroke(Stroke::new(1.2_f32, Color32::from_rgb(55, 80, 120)))
            .rounding(Rounding::same(8.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ic::CODE).size(26.0).color(Color32::from_rgb(100, 180, 255)));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("INTEGRACIÓN DIRECTA CON UNDERTALE MOD TOOL (UTMT)")
                                .size(15.0)
                                .strong()
                                .color(Color32::from_rgb(220, 235, 255)),
                        );
                        ui.label(
                            RichText::new("Abre UTMT con data.win ya precargado, explora los scripts CSX o accede directamente a los binarios del juego para auditoría e inyección manual.")
                                .size(11.5)
                                .color(Color32::from_gray(170)),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_utmt = egui::Button::new(
                            RichText::new(format!("{}  Abrir UTMT con data.win", ic::CODE))
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(35, 75, 135))
                        .min_size(Vec2::new(180.0, 32.0));

                        if ui.add(btn_utmt).clicked() {
                            match GameLauncher::open_undertale_mod_tool(&self.project.utmt_scripts_dir, &self.project.game_dir) {
                                Ok(msg) => {
                                    self.status_is_error = false;
                                    self.status_message = msg;
                                    self.show_toast(format!("{} UndertaleModTool ejecutado correctamente.", ic::CHECK_CIRCLE), false);
                                }
                                Err(err) => {
                                    self.status_is_error = true;
                                    self.status_message = err.clone();
                                    self.show_toast(format!("{} {}", ic::WARNING_CIRCLE, err), true);
                                }
                            }
                        }

                        if ui.button(format!("{}  Carpeta Scripts CSX", ic::FOLDER_OPEN)).clicked() {
                            if let Err(e) = GameLauncher::open_utmt_scripts_folder(&self.project.utmt_scripts_dir) {
                                self.show_toast(format!("{} {}", ic::WARNING_CIRCLE, e), true);
                            }
                        }

                        if ui.button(format!("{}  Carpeta del Juego", ic::FOLDER_OPEN)).clicked() {
                            if let Err(e) = GameLauncher::open_game_folder(&self.project.game_dir) {
                                self.show_toast(format!("{} {}", ic::WARNING_CIRCLE, e), true);
                            }
                        }
                    });
                });
            });

        // 6. NOTA TÉCNICA DE MODIFICACIONES Y AUDITORÍA DE DATA.WIN
        ui.add_space(8.0);
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(24, 25, 32))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 80)))
            .rounding(Rounding::same(8.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ic::SHIELD_CHECK).size(24.0).color(Color32::from_rgb(75, 210, 140)));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("NOTA TÉCNICA: ESTADO DE MODIFICACIONES EN DATA.WIN")
                            .size(14.5)
                            .strong()
                            .color(Color32::WHITE),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let doc_btn = egui::Button::new(
                            RichText::new(format!("{}  Ver Auditoría Completa (.md)", ic::ARROW_SQUARE_OUT))
                                .strong()
                                .color(Color32::from_rgb(180, 225, 255)),
                        )
                        .fill(Color32::from_rgb(35, 50, 70));

                        if ui.add(doc_btn).on_hover_text("Abrir AUDITORIA_MODIFICACIONES_GAME.md en el editor").clicked() {
                            if let Err(e) = GameLauncher::open_audit_note(&self.project.workspace_dir) {
                                self.show_toast(format!("{} {}", ic::WARNING_CIRCLE, e), true);
                            }
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                egui::Grid::new("audit_summary_grid")
                    .num_columns(2)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Estado de data.win:").strong().color(Color32::from_rgb(130, 200, 255)));
                        ui.label(RichText::new("INTACTO / ORIGINAL (26/10/2026 13:22) — Los scripts operan como inyecciones controladas.").color(Color32::from_gray(200)));
                        ui.end_row();

                        ui.label(RichText::new("Phantoms & Marionette:").strong().color(Color32::from_rgb(80, 220, 130)));
                        ui.label(RichText::new("NO AFECTADOS POR MODS. oMarionette (ID 100038) y oPhantoms (ID 100060) son nativos de la Sala 3 (Office).").color(Color32::from_gray(200)));
                        ui.end_row();

                        ui.label(RichText::new("Four Layers Down:").strong().color(Color32::from_rgb(255, 210, 100)));
                        ui.label(RichText::new("Modo Endless 100% nativo. 'The Phantoms' en la tienda/selector incrementa AmountOfPhantoms y Aggro.").color(Color32::from_gray(200)));
                        ui.end_row();

                        ui.label(RichText::new("Objetos Murray Añadidos:").strong().color(Color32::from_rgb(245, 150, 60)));
                        ui.label(RichText::new("oMurray_BackWindow (persianas independientes), oMurray_CustomNightTile (scroll TreeY), oMurray_DeathScreen.").color(Color32::from_gray(200)));
                        ui.end_row();

                        ui.label(RichText::new("Invariantes de Seguridad:").strong().color(Color32::from_rgb(210, 160, 255)));
                        ui.label(RichText::new("Variables inicializadas en línea 1 de Create_0, compatibilidad total con póster de Sun, sin sobrescritura de scripts.").color(Color32::from_gray(200)));
                        ui.end_row();
                    });
            });
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
            if ui.button(ic::FOLDER_OPEN).on_hover_text("Seleccionar carpeta").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    *target = p;
                }
            }
            if ui.button(ic::ARROW_SQUARE_OUT).on_hover_text("Abrir en Explorador de Windows").clicked() {
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("explorer").arg(&target).spawn();
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
    fn show_toast(&mut self, message: impl Into<String>, is_error: bool) {
        let msg = message.into();
        self.status_message = msg.clone();
        self.status_is_error = is_error;
        self.toast_message = Some((msg, is_error, 3.5));
    }

    fn teleport_player(&mut self, x: f32, y: f32, yaw: f32, zone_title: &str) {
        self.maze_player_x = x;
        self.maze_player_y = y;
        self.maze_player_yaw = yaw;
        self.show_toast(format!("Teletransportado a: {}", zone_title), false);
    }

    fn reset_maze_player_spawn(&mut self) {
        let (x, y, yaw) = match self.selected_salvage_location {
            SalvageLocationIndex::Location1Fnaf1 => (960.0, 224.0, 90.0),
            SalvageLocationIndex::Location2Fnaf2 => (1760.0, 336.0, 180.0),
            SalvageLocationIndex::Location3Fnaf3 => (1280.0, 192.0, 90.0),
            SalvageLocationIndex::Location4WilliamWoods => (752.0, 64.0, 90.0),
            SalvageLocationIndex::Location5SisterLocation => (1232.0, 1200.0, 270.0),
        };
        self.maze_player_x = x;
        self.maze_player_y = y;
        self.maze_player_yaw = yaw;
        self.salvage_cam_distance = 0.0;
        self.salvage_cam_yaw = 0.0;
    }

    fn get_maze_texture(&mut self, ctx: &egui::Context, tex_name: &str) -> Option<egui::TextureHandle> {
        let clean = tex_name.trim_end_matches(".png");
        let candidates = [
            PathBuf::from("assets/textures").join(format!("{}.png", clean)),
            PathBuf::from("assets/textures").join(format!("{}_0.png", clean)),
            PathBuf::from("assets/maze_textures").join(format!("{}.png", clean)),
            PathBuf::from("assets/maze_textures").join(format!("{}_0.png", clean)),
            PathBuf::from(r"D:\Frickbears-mod\Maze_Textures").join(format!("{}.png", clean)),
            PathBuf::from(r"D:\Frickbears-mod\Maze_Textures").join(format!("{}_0.png", clean)),
        ];

        for path in &candidates {
            if path.exists() {
                if let Some(tex) = self.get_or_load_tex(ctx, path, 512) {
                    return Some(tex);
                }
            }
        }
        None
    }

    fn get_texture_for_instance(&mut self, ctx: &egui::Context, inst: &MazeRoomInstance, location: SalvageLocationIndex) -> Option<egui::TextureHandle> {
        let obj = &inst.object;
        let cabinet_tex = match ((inst.x / 32).abs() + (inst.y / 32).abs()) % 4 {
            0 => "sMazeTex_ArcadeCabinet_Freddy",
            1 => "sMazeTex_ArcadeCabinet_Bonnie",
            2 => "sMazeTex_ArcadeCabinet_Chica",
            _ => "sMazeTex_ArcadeCabinet_Foxy",
        };

        let tex_name = match location {
            SalvageLocationIndex::Location3Fnaf3 => {
                if obj.contains("Wall_Vent") || obj.contains("VentSeal") || obj.contains("oMazeVent") {
                    "sMazeWall_Vent"
                } else if obj.contains("PartsBox") {
                    "sMazeTex_PartsBox"
                } else if obj.contains("GiftBox") {
                    "sMazeTex_GiftBox"
                } else if obj.contains("Arcade") {
                    cabinet_tex
                } else if obj.contains("Floor_Concrete") {
                    "sMazeTex_FloorConcrete"
                } else if obj.contains("Floor_Vent") {
                    "sMazeTex_SLVentWall"
                } else if obj.contains("Sign") || obj.contains("Poster") {
                    "sMazeDecor_OfficePosters"
                } else if obj.contains("Table") {
                    "sMazeTex_OfficeTable"
                } else {
                    "sMazeTex_Wood"
                }
            },
            SalvageLocationIndex::Location4WilliamWoods => {
                if obj.contains("Wall_Trees") || obj.contains("Tree") {
                    "sMazeWall_Trees"
                } else if obj.contains("Fence") {
                    "sMazeWall_Fence"
                } else if obj.contains("HouseBedroom") || obj.contains("Bed") {
                    "sMazeWall_HouseBedroom"
                } else if obj.contains("HouseHallway") {
                    "sMazeWall_HouseHallway"
                } else if obj.contains("HouseExterior") {
                    "sMazeWall_HouseExterior"
                } else if obj.contains("HouseGarage") {
                    "sMazeWall_HouseGarage"
                } else if obj.contains("HouseLivingRoom") {
                    "sMazeWall_HouseLivingRoom"
                } else if obj.contains("Curtain") {
                    "sMazeWall_Curtain"
                } else if obj.contains("BedThin") {
                    "sMazeTex_BedThin"
                } else if obj.contains("Bed") {
                    "sMazeTex_Bed"
                } else if obj.contains("Dresser") {
                    "sMazeTex_Dresser2"
                } else if obj.contains("MikeTV") {
                    "sMazeTex_MikeTV"
                } else if obj.contains("Sofa") {
                    "sMazeTex_Sofa"
                } else if obj.contains("Chair") {
                    "sMazeTex_Chair"
                } else if obj.contains("Fridge") {
                    "sMazeTex_Fridge"
                } else if obj.contains("Oven") {
                    "sMazeTex_Oven"
                } else if obj.contains("Counter") || obj.contains("Cupboard") {
                    "sMazeTex_KitchenCounter"
                } else if obj.contains("Window") {
                    "sMazeTex_Window"
                } else {
                    if inst.x > 900 { "sMazeWall_Trees" } else { "sMazeWall_HouseHallway" }
                }
            },
            SalvageLocationIndex::Location5SisterLocation => {
                if obj.contains("Scooping") {
                    "sMazeWall_ScoopingRoom"
                } else if obj.contains("Breaker") {
                    "sMazeWall_BreakerRoom"
                } else if obj.contains("CircusGallery") || obj.contains("CircusControls") {
                    "sMazeTex_CircusGalleryC"
                } else if obj.contains("ControlModule") {
                    "sMazeTex_ControlModule"
                } else if obj.contains("Elevator") {
                    "sMazeTex_Elevator"
                } else if obj.contains("Scooper") {
                    "sMazeDecor_Scooper"
                } else {
                    "sMazeTex_SLVentWall"
                }
            },
            SalvageLocationIndex::Location2Fnaf2 => {
                if obj.contains("Carousel") {
                    "sMazeTex_CarouselHorse"
                } else if obj.contains("Puppet") {
                    "sMazeTex_PuppetBox"
                } else if obj.contains("GiftBox") {
                    "sMazeTex_GiftBox"
                } else if obj.contains("Poster") || obj.contains("Sign") {
                    "sMazeDecor_PosterToys1"
                } else if obj.contains("Table") {
                    "sMazeTex_OfficeTable"
                } else if obj.contains("Arcade") {
                    cabinet_tex
                } else if obj.contains("Balloons") {
                    "sMazeDecor_Balloons"
                } else if obj.contains("PaperPals") {
                    "sMazeDecor_PaperPals"
                } else if obj.contains("MonitorTiny") {
                    "sMazeTex_MonitorTiny"
                } else if obj.contains("Monitor") {
                    "sMazeTex_Monitor"
                } else if obj.contains("Marionette") {
                    "sMazeTex_Marionette"
                } else if obj.contains("Vent") {
                    "sMazeWall_Vent"
                } else {
                    "sMazeWall"
                }
            },
            SalvageLocationIndex::Location1Fnaf1 => {
                if obj.contains("Arcade") {
                    cabinet_tex
                } else if obj.contains("Poster") {
                    "sMazeDecor_PosterFreddy"
                } else if obj.contains("Hatchet") {
                    "sMazeFloorDecoration_Hatchet"
                } else if obj.contains("Door") {
                    "sMazeDoor"
                } else if obj.contains("Table") {
                    "sMazeTex_OfficeTable"
                } else if obj.contains("Shelf") {
                    "sMazeTex_ShelfWall"
                } else if obj.contains("Speaker") {
                    "sMazeTex_Speaker"
                } else if obj.contains("Chair") {
                    "sMazeTex_Chair"
                } else if obj.contains("Flooring") || obj.contains("Floor") {
                    "sMazeTex_Flooring"
                } else {
                    "sMazeWall_Tiles"
                }
            },
        };

        self.get_maze_texture(ctx, tex_name)
    }

    // --------------------------------------------------------------
    // PESTAÑA 6: VISOR & DEBUGGER DE LOCACIONES 3D (RECREACIÓN 1:1 REAL)
    // --------------------------------------------------------------
    fn ui_environment_3d_builder_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let cur_room_name = self.selected_salvage_location.room_name();
        let active_room_data = self.maze_rooms.get(cur_room_name).cloned();

        // ==============================================================
        // BARRA SUPERIOR: MODOS, PAUSA/REINICIO Y SELECTOR DE LAS 5 SALAS
        // ==============================================================
        ui.horizontal(|ui| {
            let is_sim = self.salvage_tab_mode == SalvageTabMode::LiveSimulator;
            if ui.selectable_label(is_sim, RichText::new(format!("{}  Simulador 3D (WASD 1:1)", ic::GAME_CONTROLLER)).strong().size(13.0)).clicked() {
                self.salvage_tab_mode = SalvageTabMode::LiveSimulator;
            }
            let is_bp = self.salvage_tab_mode == SalvageTabMode::BlueprintEditor;
            if ui.selectable_label(is_bp, RichText::new(format!("{}  Plano Cenital & Debugger (2D)", ic::COMPASS)).strong().size(13.0)).clicked() {
                self.salvage_tab_mode = SalvageTabMode::BlueprintEditor;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            let (pause_txt, pause_col) = if self.salvage_sim_paused {
                (format!("{} Reanudar", ic::PLAY), Color32::from_rgb(90, 225, 120))
            } else {
                (format!("{} Pausar", ic::PAUSE), Color32::from_rgb(255, 180, 50))
            };

            if ui.button(RichText::new(pause_txt).strong().color(pause_col)).on_hover_text("Pausar / Reanudar la simulación 3D").clicked() {
                self.salvage_sim_paused = !self.salvage_sim_paused;
            }

            if ui.button(format!("{} Reiniciar al Spawn", ic::ARROW_COUNTER_CLOCKWISE)).on_hover_text("Devolver jugador al spawn inicial de esta sala").clicked() {
                self.reset_maze_player_spawn();
                self.salvage_sim_paused = false;
                self.show_toast("Jugador restablecido al Spawn inicial", false);
            }

            let light_col = if self.salvage_flashlight { Color32::from_rgb(255, 230, 90) } else { Color32::GRAY };
            if ui.button(RichText::new(format!("{} Linterna", ic::FLASHLIGHT)).strong().color(light_col)).on_hover_text("Encender / Apagar la linterna [Espacio]").clicked() {
                self.salvage_flashlight = !self.salvage_flashlight;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

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
                    self.reset_maze_player_spawn();
                    self.show_toast(format!("Cargando locación: {}", loc.display_name()), false);
                }
            }
        });

        ui.add_space(4.0);

        // ==============================================================
        // FILA DE COMODIDAD & USABILIDAD: PRESETS DE TELETRANSPORTE & CONTROLES
        // ==============================================================
        if self.salvage_tab_mode == SalvageTabMode::LiveSimulator {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📍 Teletransporte:").strong().size(12.0).color(Color32::from_rgb(0, 215, 255)));

                match self.selected_salvage_location {
                    SalvageLocationIndex::Location1Fnaf1 => {
                        if ui.button("🎭 Escenario").on_hover_text("Viajar al Show Stage (Freddy, Bonnie, Chica)").clicked() {
                            self.teleport_player(960.0, 224.0, 180.0, "Escenario Principal");
                        }
                        if ui.button("🍕 Comedor").on_hover_text("Viajar al Salón de Comedor y Mesas").clicked() {
                            self.teleport_player(720.0, 480.0, 180.0, "Salón de Comedor");
                        }
                        if ui.button("🦊 Pirate Cove").on_hover_text("Viajar a la Guarida del Pirata (Foxy)").clicked() {
                            self.teleport_player(420.0, 360.0, 90.0, "Guarida del Pirata");
                        }
                        if ui.button("🚪 Pasillo Oeste").on_hover_text("Viajar al Pasillo Oeste y Almacén").clicked() {
                            self.teleport_player(360.0, 680.0, 180.0, "Pasillo Oeste");
                        }
                        if ui.button("🖥️ Oficina").on_hover_text("Viajar a la Oficina del Guardia").clicked() {
                            self.teleport_player(640.0, 960.0, 0.0, "Oficina de Seguridad");
                        }
                    },
                    SalvageLocationIndex::Location2Fnaf2 => {
                        if ui.button("🎁 Prize Corner").on_hover_text("Viajar a la Zona de Regalos y Caja de Puppet").clicked() {
                            self.teleport_player(1760.0, 336.0, 180.0, "Prize Corner (Puppet Box)");
                        }
                        if ui.button("🎠 Carrusel").on_hover_text("Viajar a la Zona del Carrusel").clicked() {
                            self.teleport_player(1232.0, 624.0, 270.0, "Zona del Carrusel");
                        }
                        if ui.button("🎉 Party Rooms").on_hover_text("Viajar a las Salas de Fiesta").clicked() {
                            self.teleport_player(540.0, 450.0, 90.0, "Salas de Fiesta");
                        }
                        if ui.button("🚪 Pasillo Central").on_hover_text("Viajar al Pasillo Central").clicked() {
                            self.teleport_player(960.0, 520.0, 180.0, "Pasillo Central");
                        }
                    },
                    SalvageLocationIndex::Location3Fnaf3 => {
                        if ui.button("🟢 Pasillo Fright").on_hover_text("Viajar al Pasillo Principal de Fazbear's Fright").clicked() {
                            self.teleport_player(1280.0, 192.0, 180.0, "Pasillo de Fazbear's Fright");
                        }
                        if ui.button("💨 Ventilaciones").on_hover_text("Viajar a los Conductos de Ventilación").clicked() {
                            self.teleport_player(1150.0, 720.0, 0.0, "Conductos de Ventilación");
                        }
                        if ui.button("📦 Desguace").on_hover_text("Viajar al Almacén de Desguace").clicked() {
                            self.teleport_player(420.0, 350.0, 90.0, "Almacén de Desguace");
                        }
                    },
                    SalvageLocationIndex::Location4WilliamWoods => {
                        if ui.button("🛏️ Habitación").on_hover_text("Viajar a la Habitación Principal (Bed & Dresser)").clicked() {
                            self.teleport_player(752.0, 64.0, 90.0, "Habitación Principal");
                        }
                        if ui.button("🌲 Bosque Sombrío").on_hover_text("Viajar al Exterior entre los Árboles").clicked() {
                            self.teleport_player(1600.0, 800.0, 180.0, "Bosque Sombrío");
                        }
                        if ui.button("🚗 Carretera").on_hover_text("Viajar a la Carretera Nocturna").clicked() {
                            self.teleport_player(1850.0, 1400.0, 270.0, "Carretera Nocturna");
                        }
                    },
                    SalvageLocationIndex::Location5SisterLocation => {
                        if ui.button("🕹️ Módulo Control").on_hover_text("Viajar al Módulo de Control y Ascensor").clicked() {
                            self.teleport_player(1232.0, 1200.0, 270.0, "Módulo de Control Primario");
                        }
                        if ui.button("🥄 Scooping Room").on_hover_text("Viajar a la Sala de Desmantelamiento (Scooper)").clicked() {
                            self.teleport_player(540.0, 640.0, 0.0, "Sala del Scooper");
                        }
                        if ui.button("🎪 Circus Gallery").on_hover_text("Viajar a la Galería del Circo de Baby").clicked() {
                            self.teleport_player(1650.0, 580.0, 180.0, "Galería del Circo");
                        }
                    },
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Selector de Velocidad
                ui.label(RichText::new("Velocidad:").size(12.0));
                if ui.selectable_label(self.move_speed_multiplier == 1.0, "1x").on_hover_text("Velocidad normal de paso").clicked() {
                    self.move_speed_multiplier = 1.0;
                }
                if ui.selectable_label(self.move_speed_multiplier == 2.0, "2x").on_hover_text("Velocidad rápida (x2)").clicked() {
                    self.move_speed_multiplier = 2.0;
                }
                if ui.selectable_label(self.move_speed_multiplier == 4.0, "4x").on_hover_text("Velocidad turbo (x4)").clicked() {
                    self.move_speed_multiplier = 4.0;
                }

                ui.add_space(6.0);
                let noclip_col = if self.noclip_mode { Color32::from_rgb(255, 140, 50) } else { Color32::GRAY };
                if ui.button(RichText::new(format!("{} Noclip", ic::GHOST)).color(noclip_col).strong()).on_hover_text("Activar / Desactivar modo fantasma (atravesar paredes sin colisión)").clicked() {
                    self.noclip_mode = !self.noclip_mode;
                    if self.noclip_mode {
                        self.show_toast("Modo Noclip ACTIVADO (Sin colisión)", false);
                    } else {
                        self.show_toast("Modo Noclip DESACTIVADO (Física sólida)", false);
                    }
                }

                ui.add_space(6.0);
                let radar_btn_txt = match self.radar_display_mode {
                    0 => "🗺️ Radar: Normal",
                    1 => "🗺️ Radar: Zoom (Grande)",
                    _ => "🗺️ Radar: Oculto",
                };
                if ui.button(radar_btn_txt).on_hover_text("Alternar tamaño del mini-mapa o esconderlo").clicked() {
                    self.radar_display_mode = (self.radar_display_mode + 1) % 3;
                }

                let cross_col = if self.show_crosshair { Color32::from_rgb(110, 220, 255) } else { Color32::GRAY };
                if ui.button(RichText::new(format!("{} Mira", ic::CROSSHAIR)).color(cross_col)).on_hover_text("Mostrar / Ocultar la retícula de inspección central").clicked() {
                    self.show_crosshair = !self.show_crosshair;
                }
            });
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(2.0);

        // ==============================================================
        // ENTRADA DE TECLADO: NAVEGACIÓN WASD / FLECHAS EN EL MAPA 1:1
        // ==============================================================
        if self.salvage_tab_mode == SalvageTabMode::LiveSimulator && !self.salvage_sim_paused {
            ctx.input(|i| {
                let turn_speed = 2.4_f32;
                let base_speed = if i.modifiers.shift { 6.8_f32 } else { 3.4_f32 };
                let move_speed = base_speed * self.move_speed_multiplier;

                // Rotación de Cámara (Rumbo / Yaw)
                if i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft) {
                    self.maze_player_yaw = (self.maze_player_yaw - turn_speed).rem_euclid(360.0);
                }
                if i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight) {
                    self.maze_player_yaw = (self.maze_player_yaw + turn_speed).rem_euclid(360.0);
                }

                // Linterna
                if i.key_pressed(egui::Key::Space) {
                    self.salvage_flashlight = !self.salvage_flashlight;
                }

                // Vector de avance según rumbo (rad)
                let rad = self.maze_player_yaw.to_radians();
                let fwd_x = rad.cos();
                let fwd_y = rad.sin();

                let mut m_dx = 0.0_f32;
                let mut m_dy = 0.0_f32;

                if i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp) {
                    m_dx += fwd_x * move_speed;
                    m_dy += fwd_y * move_speed;
                }
                if i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown) {
                    m_dx -= fwd_x * move_speed;
                    m_dy -= fwd_y * move_speed;
                }

                let target_x = self.maze_player_x + m_dx;
                let target_y = self.maze_player_y + m_dy;

                // Comprobación de colisión física (Omitida si Noclip está activo)
                if self.noclip_mode {
                    self.maze_player_x = target_x;
                    self.maze_player_y = target_y;
                } else {
                    let mut blocked_x = false;
                    let mut blocked_y = false;

                    if let Some(rd) = &active_room_data {
                        for inst in &rd.instances {
                            if inst.object.contains("Invisible") { continue; }
                            let is_obstacle = inst.object.contains("Wall")
                                || inst.object.contains("Arcade")
                                || inst.object.contains("Table")
                                || inst.object.contains("Door")
                                || inst.object.contains("Bed")
                                || inst.object.contains("Sofa")
                                || inst.object.contains("Shelf")
                                || inst.object.contains("Dresser")
                                || inst.object.contains("Counter")
                                || inst.object.contains("Cupboard")
                                || inst.object.contains("Oven");
                            if is_obstacle {
                                let pad = 12.0_f32;
                                let hw = inst.sx * 16.0;
                                let hh = inst.sy * 16.0;
                                let wx0 = inst.x as f32 - hw - pad;
                                let wx1 = inst.x as f32 + hw + pad;
                                let wy0 = inst.y as f32 - hh - pad;
                                let wy1 = inst.y as f32 + hh + pad;
                                if target_x >= wx0 && target_x <= wx1 && self.maze_player_y >= wy0 && self.maze_player_y <= wy1 {
                                    blocked_x = true;
                                }
                                if self.maze_player_x >= wx0 && self.maze_player_x <= wx1 && target_y >= wy0 && target_y <= wy1 {
                                    blocked_y = true;
                                }
                            }
                        }
                    }

                    if !blocked_x { self.maze_player_x = target_x; }
                    if !blocked_y { self.maze_player_y = target_y; }
                }
            });
        }

        // Determinar límites de la sala actual para escala del Radar
        let (min_x, max_x, min_y, max_y) = match self.selected_salvage_location {
            SalvageLocationIndex::Location1Fnaf1 => (-256.0_f32, 1416.0_f32, -112.0_f32, 1256.0_f32),
            SalvageLocationIndex::Location2Fnaf2 => (16.0_f32, 1840.0_f32, 0.0_f32, 1176.0_f32),
            SalvageLocationIndex::Location3Fnaf3 => (-16.0_f32, 1496.0_f32, -16.0_f32, 1000.0_f32),
            SalvageLocationIndex::Location4WilliamWoods => (144.0_f32, 2136.0_f32, -16.0_f32, 1616.0_f32),
            SalvageLocationIndex::Location5SisterLocation => (-16.0_f32, 2128.0_f32, -16.0_f32, 1328.0_f32),
        };

        // Identificación de la Zona Actual del Restaurante
        let zone_name = match self.selected_salvage_location {
            SalvageLocationIndex::Location1Fnaf1 => {
                if self.maze_player_x > 800.0 && self.maze_player_y < 350.0 {
                    "Escenario Principal (Show Stage)"
                } else if self.maze_player_x > 550.0 && self.maze_player_y >= 350.0 && self.maze_player_y < 700.0 {
                    "Salón de Comedor y Mesas (Dining Area)"
                } else if self.maze_player_x <= 550.0 && self.maze_player_y < 500.0 {
                    "Guarida del Pirata (Pirate Cove)"
                } else if self.maze_player_x < 500.0 && self.maze_player_y >= 500.0 {
                    "Pasillo Oeste & Almacén (West Hall)"
                } else if self.maze_player_x >= 500.0 && self.maze_player_y >= 700.0 {
                    "Oficina del Guardia de Seguridad (The Office)"
                } else {
                    "Pasillo Este & Área de Cocina"
                }
            },
            SalvageLocationIndex::Location2Fnaf2 => {
                if self.maze_player_x > 1400.0 && self.maze_player_y < 500.0 {
                    "Zona de Regalos & Puppet Box (Prize Corner)"
                } else if self.maze_player_x > 1000.0 && self.maze_player_y >= 500.0 {
                    "Zona del Carrusel de Fantasía (Carousel Area)"
                } else if self.maze_player_x < 800.0 {
                    "Salas de Fiesta 1 a 4 (Party Rooms)"
                } else {
                    "Pasillo Central de FNAF 2"
                }
            },
            SalvageLocationIndex::Location3Fnaf3 => {
                if self.maze_player_y < 400.0 {
                    "Pasillo Principal de Atracción (Fazbear's Fright)"
                } else if self.maze_player_x > 1000.0 {
                    "Conductos y Ventilaciones Ocultas"
                } else {
                    "Salas de Desguace y Props Antiguos"
                }
            },
            SalvageLocationIndex::Location4WilliamWoods => {
                if self.maze_player_y < 500.0 {
                    "Habitación del Niño (Bed & Dresser)"
                } else if self.maze_player_x < 900.0 {
                    "Pasillo de la Casa de Medianoche"
                } else {
                    "Exterior / Bosque Sombrío (William's Woods)"
                }
            },
            SalvageLocationIndex::Location5SisterLocation => {
                if self.maze_player_y > 1000.0 {
                    "Módulo de Control Primario (Elevator & Controls)"
                } else if self.maze_player_x < 800.0 {
                    "Sala de Desmantelamiento (Scooping Room)"
                } else if self.maze_player_x > 1400.0 {
                    "Galería del Circo de Baby (Circus Gallery)"
                } else {
                    "Conductos Subterráneos de Mantenimiento"
                }
            },
        };

        if self.salvage_tab_mode == SalvageTabMode::LiveSimulator {
            // ==============================================================
            // MODO 1: SIMULADOR 3D RECREACIÓN 1:1 CON INSTANCIAS REALES
            // ==============================================================
            let avail_w = ui.available_width().max(600.0);
            let view_h = 440.0;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, view_h), Sense::click());
            let painter = ui.painter_at(rect);

            // Cargar texturas del mapa para proyectar en 3D (1:1 con GameMaker)
            let tex_floor = match self.selected_salvage_location {
                SalvageLocationIndex::Location1Fnaf1 => self.get_maze_texture(ctx, "sMazeTex_Flooring"),
                SalvageLocationIndex::Location2Fnaf2 => self.get_maze_texture(ctx, "sMazeTex_FlooringPanels"),
                SalvageLocationIndex::Location3Fnaf3 => self.get_maze_texture(ctx, "sMazeTex_FloorConcrete").or_else(|| self.get_maze_texture(ctx, "sMazeTex_Wood")),
                SalvageLocationIndex::Location4WilliamWoods => self.get_maze_texture(ctx, "sMazeTex_FlooringPanels"),
                SalvageLocationIndex::Location5SisterLocation => self.get_maze_texture(ctx, "sMazeTex_FloorConveyor").or_else(|| self.get_maze_texture(ctx, "sMazeTex_FloorConcrete")),
            };
            let tex_ceiling = self.get_maze_texture(ctx, "sMazeTex_Ceiling");
            let tex_wood = self.get_maze_texture(ctx, "sMazeTex_Wood");
            let tex_default_wall = match self.selected_salvage_location {
                SalvageLocationIndex::Location1Fnaf1 => self.get_maze_texture(ctx, "sMazeWall_Tiles"),
                SalvageLocationIndex::Location2Fnaf2 => self.get_maze_texture(ctx, "sMazeWall"),
                SalvageLocationIndex::Location3Fnaf3 => self.get_maze_texture(ctx, "sMazeTex_Wood"),
                SalvageLocationIndex::Location4WilliamWoods => self.get_maze_texture(ctx, "sMazeWall_HouseHallway"),
                SalvageLocationIndex::Location5SisterLocation => self.get_maze_texture(ctx, "sMazeTex_SLVentWall"),
            };

            // Parámetros de Cámara y Perspectiva 1:1 con GameMaker
            let horizon_y = rect.center().y;
            let rad = self.maze_player_yaw.to_radians();
            let cos_yaw = rad.cos();
            let sin_yaw = rad.sin();
            let fov = 64.0_f32.to_radians(); // FOV 64 auténtico de oCamera3DMaze
            let half_fov_tan = (fov * 0.5).tan();
            let focal = (rect.width() * 0.5) / half_fov_tan;

            // Fondo y Niebla Profunda
            let sky_color = match self.selected_salvage_location {
                SalvageLocationIndex::Location1Fnaf1 => Color32::from_rgb(10, 8, 7),
                SalvageLocationIndex::Location2Fnaf2 => Color32::from_rgb(12, 10, 18),
                SalvageLocationIndex::Location3Fnaf3 => Color32::from_rgb(8, 14, 10),
                SalvageLocationIndex::Location4WilliamWoods => Color32::from_rgb(6, 8, 14),
                SalvageLocationIndex::Location5SisterLocation => Color32::from_rgb(8, 12, 20),
            };
            painter.rect_filled(rect, 4.0, sky_color);

            let flash_intensity = if self.salvage_flashlight { 1.0_f32 } else { 0.16_f32 };

            // 1. TECHO EN PERSPECTIVA 1:1 (Altura 80.0 unidades sobre la cámara: depth 128 - cam 48)
            let ceil_strips = 18;
            let ceil_total_h = horizon_y - rect.min.y;
            for i in 0..ceil_strips {
                let t0 = (i as f32 / ceil_strips as f32).powf(1.6);
                let t1 = ((i + 1) as f32 / ceil_strips as f32).powf(1.6);
                let y0 = rect.min.y + (t0 * ceil_total_h);
                let y1 = rect.min.y + (t1 * ceil_total_h);
                let y_mid = (y0 + y1) * 0.5;
                let dy = (horizon_y - y_mid).max(1.0);
                let d = (80.0 * focal) / dy; // 80 unidades reales

                let depth_factor = (1.0 - (d / 650.0)).clamp(0.0, 1.0);
                let brightness = (depth_factor * flash_intensity).clamp(0.08, 0.65);
                let col = Color32::from_rgba_unmultiplied(
                    (255.0 * brightness) as u8,
                    (255.0 * brightness) as u8,
                    (255.0 * brightness) as u8,
                    255,
                );

                let strip_rect = Rect::from_min_max(Pos2::new(rect.min.x, y0), Pos2::new(rect.max.x, y1));
                if let Some(ct) = &tex_ceiling {
                    let span_w = (rect.width() / focal) * d;
                    let cx = self.maze_player_x + d * cos_yaw;
                    let cy = self.maze_player_y + d * sin_yaw;
                    let rx = -sin_yaw;
                    let ry = cos_yaw;
                    let lx = cx - 0.5 * span_w * rx;
                    let ly = cy - 0.5 * span_w * ry;
                    let rx_w = cx + 0.5 * span_w * rx;
                    let ry_w = cy + 0.5 * span_w * ry;

                    let mut mesh = egui::Mesh::with_texture(ct.id());
                    let idx = mesh.vertices.len() as u32;
                    let u_l = lx / 32.0;
                    let v_l = ly / 32.0;
                    let u_r = rx_w / 32.0;
                    let v_r = ry_w / 32.0;

                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, y0), uv: Pos2::new(u_l, v_l), color: col });
                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, y0), uv: Pos2::new(u_r, v_r), color: col });
                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, y1), uv: Pos2::new(u_r, v_r + 0.25), color: col });
                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, y1), uv: Pos2::new(u_l, v_l + 0.25), color: col });
                    mesh.add_triangle(idx, idx + 1, idx + 2);
                    mesh.add_triangle(idx, idx + 2, idx + 3);
                    painter.add(egui::Shape::mesh(mesh));
                } else {
                    painter.rect_filled(strip_rect, 0.0, col);
                }
            }

            // 2. SUELO EN PERSPECTIVA SUAVE Y REALISTA (Altura 48.0 unidades bajo la cámara)
            let floor_strips = 24;
            let floor_total_h = rect.max.y - horizon_y;
            for i in 0..floor_strips {
                let t0 = (i as f32 / floor_strips as f32).powf(1.7);
                let t1 = ((i + 1) as f32 / floor_strips as f32).powf(1.7);
                let y0 = horizon_y + (t0 * floor_total_h);
                let y1 = horizon_y + (t1 * floor_total_h);
                let y_mid = (y0 + y1) * 0.5;
                let dy = (y_mid - horizon_y).max(1.0);
                let d = (48.0 * focal) / dy; // 48 unidades reales

                let dist_bright = ((1.0 - (d / 650.0)).clamp(0.0, 1.0) * flash_intensity).clamp(0.12, 1.0);
                let col = Color32::from_rgba_unmultiplied(
                    (255.0 * dist_bright) as u8,
                    (255.0 * dist_bright) as u8,
                    (255.0 * dist_bright) as u8,
                    255,
                );

                let strip_rect = Rect::from_min_max(Pos2::new(rect.min.x, y0), Pos2::new(rect.max.x, y1));
                if let Some(ft) = &tex_floor {
                    let span_w = (rect.width() / focal) * d;
                    let cx = self.maze_player_x + d * cos_yaw;
                    let cy = self.maze_player_y + d * sin_yaw;
                    let rx = -sin_yaw;
                    let ry = cos_yaw;
                    let lx = cx - 0.5 * span_w * rx;
                    let ly = cy - 0.5 * span_w * ry;
                    let rx_w = cx + 0.5 * span_w * rx;
                    let ry_w = cy + 0.5 * span_w * ry;

                    let mut mesh = egui::Mesh::with_texture(ft.id());
                    let idx = mesh.vertices.len() as u32;
                    let u_l = lx / 32.0;
                    let v_l = ly / 32.0;
                    let u_r = rx_w / 32.0;
                    let v_r = ry_w / 32.0;

                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, y0), uv: Pos2::new(u_l, v_l), color: col });
                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, y0), uv: Pos2::new(u_r, v_r), color: col });
                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, y1), uv: Pos2::new(u_r, v_r + 0.25), color: col });
                    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, y1), uv: Pos2::new(u_l, v_l + 0.25), color: col });
                    mesh.add_triangle(idx, idx + 1, idx + 2);
                    mesh.add_triangle(idx, idx + 2, idx + 3);
                    painter.add(egui::Shape::mesh(mesh));
                } else {
                    let base_col = match self.selected_salvage_location {
                        SalvageLocationIndex::Location1Fnaf1 => Color32::from_rgb(22, 18, 16),
                        SalvageLocationIndex::Location2Fnaf2 => Color32::from_rgb(20, 16, 28),
                        SalvageLocationIndex::Location3Fnaf3 => Color32::from_rgb(18, 24, 20),
                        SalvageLocationIndex::Location4WilliamWoods => Color32::from_rgb(12, 16, 20),
                        SalvageLocationIndex::Location5SisterLocation => Color32::from_rgb(16, 22, 32),
                    };
                    painter.rect_filled(strip_rect, 0.0, base_col);
                }
            }

            // 3. RECOLECCIÓN DE GEOMETRÍA SÓLIDA CON CENTRADO REAL DE GAMEMAKER
            struct WallSegment {
                x1: f32,
                y1: f32,
                x2: f32,
                y2: f32,
                len: f32,
                h_top: f32,
                h_bot: f32,
                tex_id: Option<egui::TextureId>,
                obj_name: String,
                coords: (i32, i32),
                face_type: u8, // 0: estándar, 1: arcade front, 2: arcade side
            }

            let mut wall_segments: Vec<WallSegment> = Vec::new();

            struct PropBillboard {
                pz: f32,
                pcx: f32,
                screen_x: f32,
                rect: Rect,
                col: Color32,
                tex_id: Option<egui::TextureId>,
                obj_name: String,
                coords: (i32, i32),
            }
            let mut props: Vec<PropBillboard> = Vec::new();

            if let Some(rd) = &active_room_data {
                for inst in &rd.instances {
                    if inst.object.contains("Invisible") {
                        continue;
                    }

                    let obj = &inst.object;
                    let is_wall = obj.contains("Wall");
                    let is_arcade = obj.contains("Arcade");
                    let is_table = obj.contains("Table");
                    let is_door = obj.contains("Door");
                    let is_counter = obj.contains("Counter") || obj.contains("Cupboard") || obj.contains("Oven");
                    let is_bed = obj.contains("Bed") || obj.contains("Sofa");
                    let is_shelf = obj.contains("Shelf") || obj.contains("Dresser") || obj.contains("PartsBox");

                    if is_wall || is_arcade || is_table || is_door || is_counter || is_bed || is_shelf {
                        // En GameMaker: (x, y) es el CENTRO del bloque
                        let hw = inst.sx * 16.0;
                        let hh = inst.sy * 16.0;
                        let cx = inst.x as f32;
                        let cy = inst.y as f32;
                        let x0 = cx - hw;
                        let x1 = cx + hw;
                        let y0 = cy - hh;
                        let y1 = cy + hh;

                        let (h_top, h_bot) = if is_wall {
                            (80.0_f32, 48.0_f32)
                        } else if is_arcade {
                            (32.0_f32, 48.0_f32) // Altura total 80: top 32 sobre cámara, bot 48 en suelo
                        } else if is_door {
                            (48.0_f32, 48.0_f32)
                        } else if is_table {
                            (-16.0_f32, 48.0_f32)
                        } else if is_counter {
                            (-12.0_f32, 48.0_f32)
                        } else if is_bed {
                            (-16.0_f32, 48.0_f32)
                        } else if is_shelf {
                            (16.0_f32, 48.0_f32)
                        } else {
                            (80.0_f32, 48.0_f32)
                        };

                        let tex = self.get_texture_for_instance(ctx, inst, self.selected_salvage_location)
                            .or_else(|| if is_wall { tex_default_wall.clone().or_else(|| tex_wood.clone()) } else { None });
                        let tex_id = tex.as_ref().map(|t| t.id());
                        let coords = (inst.x, inst.y);
                        let w_len = x1 - x0;
                        let h_len = y1 - y0;

                        // Para máquinas recreativas: Norte y Sur son laterales (face_type 2), Oeste es frontal (face_type 1)
                        let (fnorth, fsouth, fwest, feast) = if is_arcade {
                            (2, 2, 1, 0)
                        } else {
                            (0, 0, 0, 0)
                        };

                        wall_segments.push(WallSegment { x1: x0, y1: y0, x2: x1, y2: y0, len: w_len, h_top, h_bot, tex_id, obj_name: obj.clone(), coords, face_type: fnorth });
                        wall_segments.push(WallSegment { x1: x0, y1: y1, x2: x1, y2: y1, len: w_len, h_top, h_bot, tex_id, obj_name: obj.clone(), coords, face_type: fsouth });
                        wall_segments.push(WallSegment { x1: x0, y1: y0, x2: x0, y2: y1, len: h_len, h_top, h_bot, tex_id, obj_name: obj.clone(), coords, face_type: fwest });
                        wall_segments.push(WallSegment { x1: x1, y1: y0, x2: x1, y2: y1, len: h_len, h_top, h_bot, tex_id, obj_name: obj.clone(), coords, face_type: feast });
                    } else {
                        // Billboards 2D planos reales (Cajas de regalo, marioneta, globos)
                        let px = inst.x as f32;
                        let py = inst.y as f32;
                        let dx = px - self.maze_player_x;
                        let dy = py - self.maze_player_y;

                        let pz = dx * cos_yaw + dy * sin_yaw;
                        let pcx = -dx * sin_yaw + dy * cos_yaw;

                        if pz > 8.0 && pz < 700.0 {
                            let screen_x = rect.center().x + (pcx / pz) * focal;
                            let (prop_w_world, prop_h_world) = if obj.contains("Puppet") || obj.contains("GiftBox") {
                                (32.0_f32, 36.0_f32)
                            } else if obj.contains("Balloons") {
                                (24.0_f32, 48.0_f32)
                            } else {
                                (28.0_f32, 32.0_f32)
                            };

                            let prop_w = (prop_w_world / pz) * focal;
                            let prop_h = (prop_h_world / pz) * focal;
                            let ground_y = horizon_y + (48.0 / pz) * focal;

                            let p_rect = Rect::from_min_max(
                                Pos2::new(screen_x - (prop_w * 0.5), ground_y - prop_h),
                                Pos2::new(screen_x + (prop_w * 0.5), ground_y)
                            );

                            let dist_shade = ((1.0 - (pz / 650.0)).clamp(0.0, 1.0) * flash_intensity).clamp(0.12, 1.0);
                            let col = Color32::from_rgba_unmultiplied((255.0 * dist_shade) as u8, (255.0 * dist_shade) as u8, (255.0 * dist_shade) as u8, 255);
                            let tex = self.get_texture_for_instance(ctx, inst, self.selected_salvage_location);
                            let tex_id = tex.as_ref().map(|t| t.id());

                            props.push(PropBillboard {
                                pz,
                                pcx,
                                screen_x,
                                rect: p_rect,
                                col,
                                tex_id,
                                obj_name: obj.clone(),
                                coords: (inst.x, inst.y),
                            });
                        }
                    }
                }
            }

            let num_rays = 240;
            let col_w = rect.width() / num_rays as f32;
            let mut z_buffer: Vec<f32> = vec![9999.0; num_rays];
            let mut targeted_object: Option<(String, f32, (i32, i32))> = None;

            struct Hit {
                dist: f32,
                u: f32,
                len: f32,
                h_top: f32,
                h_bot: f32,
                tex_id: Option<egui::TextureId>,
                obj_name: String,
                coords: (i32, i32),
                face_type: u8,
            }

            for r in 0..num_rays {
                let rel_x = (2.0 * r as f32 / num_rays as f32) - 1.0;
                let ray_angle = rad + (rel_x * half_fov_tan).atan();
                let rdx = ray_angle.cos();
                let rdy = ray_angle.sin();

                let mut hits: Vec<Hit> = Vec::new();

                for seg in &wall_segments {
                    let rxs = rdx * (seg.y2 - seg.y1) - rdy * (seg.x2 - seg.x1);
                    if rxs.abs() < 1e-6 { continue; }
                    let qpx = seg.x1 - self.maze_player_x;
                    let qpy = seg.y1 - self.maze_player_y;
                    let t = (qpx * (seg.y2 - seg.y1) - qpy * (seg.x2 - seg.x1)) / rxs;
                    let u = (qpx * rdy - qpy * rdx) / rxs;

                    if t > 0.1 && u >= 0.0 && u <= 1.0 && t < 700.0 {
                        hits.push(Hit {
                            dist: t,
                            u,
                            len: seg.len,
                            h_top: seg.h_top,
                            h_bot: seg.h_bot,
                            tex_id: seg.tex_id,
                            obj_name: seg.obj_name.clone(),
                            coords: seg.coords,
                            face_type: seg.face_type,
                        });
                    }
                }

                if hits.is_empty() {
                    continue;
                }

                // El hit más cercano define la profundidad en z_buffer para billboards y retícula
                if let Some(closest) = hits.iter().min_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap()) {
                    let angle_diff = ray_angle - rad;
                    let perp_dist = closest.dist * angle_diff.cos();
                    z_buffer[r] = perp_dist;

                    if (r as i32 - num_rays as i32 / 2).abs() <= 2 {
                        targeted_object = Some((closest.obj_name.clone(), perp_dist, closest.coords));
                    }
                }

                // Ordenar de LEJOS a CERCA (Painter's algorithm):
                // Paredes de fondo se dibujan primero; objetos en primer plano (recreativas, mesas)
                // se dibujan encima sin ocluir el techo de fondo.
                hits.sort_by(|a, b| b.dist.partial_cmp(&a.dist).unwrap_or(std::cmp::Ordering::Equal));

                let angle_diff = ray_angle - rad;
                let cos_diff = angle_diff.cos();
                let angle_shade = cos_diff.powi(2).clamp(0.65, 1.0);
                let col_x0 = rect.min.x + (r as f32 * col_w);
                let col_x1 = rect.min.x + ((r + 1) as f32 * col_w);

                for hit in hits {
                    let perp_dist = hit.dist * cos_diff;
                    let y0 = horizon_y - (hit.h_top / perp_dist.max(1.0)) * focal;
                    let y1 = horizon_y + (hit.h_bot / perp_dist.max(1.0)) * focal;

                    let dist_shade = ((1.0 - (perp_dist / 650.0)).clamp(0.0, 1.0) * flash_intensity).clamp(0.12, 1.0);
                    let total_shade = (dist_shade * angle_shade).clamp(0.10, 1.0);
                    let shade_u8 = (255.0 * total_shade) as u8;
                    let col = Color32::from_rgba_unmultiplied(shade_u8, shade_u8, shade_u8, 255);

                    let wall_pos = hit.u * hit.len;
                    let (u0, u1) = match hit.face_type {
                        1 => {
                            // Frontal recreativa: pantalla y marquesina en [0.5, 1.0]
                            let sub = (wall_pos.floor() as i32 % 32).rem_euclid(32) as f32;
                            let base = 0.5 + (sub / 32.0) * 0.5;
                            (base, base + (0.5 / 32.0))
                        },
                        2 => {
                            // Lateral recreativa: ilustración del personaje en [0.0, 0.5]
                            let sub = (wall_pos.floor() as i32 % 32).rem_euclid(32) as f32;
                            let base = (sub / 32.0) * 0.5;
                            (base, base + (0.5 / 32.0))
                        },
                        _ => {
                            // Textura repetida cada 32 píxeles
                            let tex_col = (wall_pos.floor() as i32 % 32).rem_euclid(32) as f32;
                            (tex_col / 32.0, (tex_col + 1.0) / 32.0)
                        }
                    };

                    if let Some(tid) = hit.tex_id {
                        let mut mesh = egui::Mesh::with_texture(tid);
                        let idx = mesh.vertices.len() as u32;
                        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(col_x0, y0), uv: Pos2::new(u0, 0.0), color: col });
                        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(col_x1, y0), uv: Pos2::new(u1, 0.0), color: col });
                        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(col_x1, y1), uv: Pos2::new(u1, 1.0), color: col });
                        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(col_x0, y1), uv: Pos2::new(u0, 1.0), color: col });
                        mesh.add_triangle(idx, idx + 1, idx + 2);
                        mesh.add_triangle(idx, idx + 2, idx + 3);
                        painter.add(egui::Shape::mesh(mesh));
                    } else {
                        painter.rect_filled(Rect::from_min_max(Pos2::new(col_x0, y0), Pos2::new(col_x1, y1)), 0.0, col);
                    }
                }
            }

            // Dibujar Props respetando z_buffer
            props.sort_by(|a, b| b.pz.partial_cmp(&a.pz).unwrap_or(std::cmp::Ordering::Equal));
            for prop in props {
                let ray_idx = ((prop.screen_x - rect.min.x) / col_w).round() as i32;
                let occluded = if ray_idx >= 0 && (ray_idx as usize) < num_rays {
                    prop.pz > (z_buffer[ray_idx as usize] + 16.0)
                } else {
                    false
                };

                if !occluded {
                    if let Some(tid) = prop.tex_id {
                        painter.image(tid, prop.rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), prop.col);
                    } else {
                        painter.rect_filled(prop.rect, 2.0, prop.col);
                    }

                    if prop.pcx.abs() < (prop.pz * 0.12) && prop.pz < 280.0 {
                        if targeted_object.as_ref().map(|(_, d, _)| prop.pz < *d).unwrap_or(true) {
                            targeted_object = Some((prop.obj_name.clone(), prop.pz, prop.coords));
                        }
                    }
                }
            }

            // Animatrónico Custom (si está habilitado)
            if self.project.salvage_placement.enabled {
                let sprite_opt = self.project.salvage_placement.salvage_sprite_path.clone();
                if let Some(p) = &sprite_opt {
                    if let Some(tex) = self.get_or_load_tex(ctx, p, 320) {
                        let dx = self.project.salvage_placement.x as f32 - self.maze_player_x;
                        let dy = self.project.salvage_placement.y as f32 - self.maze_player_y;
                        let az = dx * cos_yaw + dy * sin_yaw;
                        let acx = -dx * sin_yaw + dy * cos_yaw;

                        if az > 8.0 && az < 680.0 {
                            let screen_x = rect.center().x + (acx / az) * focal;
                            let anim_w = (48.0_f32 / az) * focal;
                            let anim_h = (56.0_f32 / az) * focal;
                            let ground_y = horizon_y + (48.0 / az.max(1.0)) * focal;
                            let a_rect = Rect::from_min_max(
                                Pos2::new(screen_x - (anim_w * 0.5), ground_y - anim_h),
                                Pos2::new(screen_x + (anim_w * 0.5), ground_y)
                            );

                            let ray_idx = ((screen_x - rect.min.x) / col_w).round() as i32;
                            let occluded = if ray_idx >= 0 && (ray_idx as usize) < num_rays {
                                az > (z_buffer[ray_idx as usize] + 20.0)
                            } else {
                                false
                            };

                            if !occluded {
                                painter.image(tex.id(), a_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                                painter.text(
                                    Pos2::new(a_rect.center().x, a_rect.min.y - 12.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("Animatrónico ({}, {})", self.project.salvage_placement.x, self.project.salvage_placement.y),
                                    egui::FontId::proportional(11.0),
                                    Color32::YELLOW,
                                );
                            }
                        }
                    }
                }
            }

            // 4. LINTERNA Y AMBIENTE DE TERROR (SIN CÍRCULOS DIANA BRUSCOS)
            if self.salvage_flashlight {
                let v_top = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + 36.0));
                let v_bot = Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - 36.0), rect.max);
                let v_left = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + 48.0, rect.max.y));
                let v_right = Rect::from_min_max(Pos2::new(rect.max.x - 48.0, rect.min.y), rect.max);
                let edge_dark = Color32::from_rgba_unmultiplied(0, 0, 0, 95);
                painter.rect_filled(v_top, 0.0, edge_dark);
                painter.rect_filled(v_bot, 0.0, edge_dark);
                painter.rect_filled(v_left, 0.0, edge_dark);
                painter.rect_filled(v_right, 0.0, edge_dark);
            } else {
                painter.rect_filled(rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 238));
            }

            // 5. Retícula Central (Crosshair) & Indicador de Inspección
            if self.show_crosshair {
                let cross_center = Pos2::new(rect.center().x, horizon_y);
                let cross_size = 8.0_f32;
                painter.line_segment([Pos2::new(cross_center.x - cross_size, cross_center.y), Pos2::new(cross_center.x + cross_size, cross_center.y)], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(100, 220, 255, 180)));
                painter.line_segment([Pos2::new(cross_center.x, cross_center.y - cross_size), Pos2::new(cross_center.x, cross_center.y + cross_size)], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(100, 220, 255, 180)));
                painter.circle_stroke(cross_center, 3.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 200)));

                if let Some((obj_name, depth, coords)) = targeted_object {
                    let clean_obj = obj_name.replace("oMazeProp_", "").replace("oMazeWall_", "").replace("oMazeFloorDecoration_", "");
                    let badge_text = format!("🎯 {} | {:.0}px | ({}, {})", clean_obj, depth, coords.0, coords.1);
                    let badge_pos = Pos2::new(cross_center.x, cross_center.y + 24.0);
                    let badge_rect = Rect::from_center_size(badge_pos, Vec2::new(badge_text.len() as f32 * 7.5 + 16.0, 22.0));
                    painter.rect_filled(badge_rect, 4.0, Color32::from_rgba_unmultiplied(12, 18, 26, 215));
                    painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(0, 200, 255)));
                    painter.text(badge_pos, egui::Align2::CENTER_CENTER, badge_text, egui::FontId::monospace(11.0), Color32::from_rgb(120, 230, 255));
                }
            }

            // 6. RADAR TOPOLÓGICO 1:1 EN LA ESQUINA SUPERIOR DERECHA (INTERACTIVO)
            if self.radar_display_mode != 2 {
                let (radar_w, radar_h) = if self.radar_display_mode == 1 {
                    (320.0_f32, 240.0_f32) // Modo Zoom Grande
                } else {
                    (210.0_f32, 160.0_f32) // Modo Normal
                };

                let radar_rect = Rect::from_min_size(Pos2::new(rect.max.x - radar_w - 12.0, rect.min.y + 12.0), Vec2::new(radar_w, radar_h));

                painter.rect_filled(radar_rect, 6.0, Color32::from_rgba_unmultiplied(12, 16, 24, 220));
                painter.rect_stroke(radar_rect, 6.0, Stroke::new(1.5_f32, Color32::from_rgb(0, 185, 230)));

                painter.text(
                    Pos2::new(radar_rect.min.x + 8.0, radar_rect.min.y + 6.0),
                    egui::Align2::LEFT_TOP,
                    format!("RADAR 1:1 ({})", cur_room_name),
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(0, 210, 255),
                );

                let to_radar = |gx: f32, gy: f32| -> Pos2 {
                    let nx = (gx - min_x) / (max_x - min_x).max(1.0);
                    let ny = (gy - min_y) / (max_y - min_y).max(1.0);
                    Pos2::new(
                        radar_rect.min.x + 8.0 + (nx * (radar_w - 16.0)),
                        radar_rect.min.y + 24.0 + (ny * (radar_h - 32.0)),
                    )
                };

                if let Some(rd) = &active_room_data {
                    for inst in &rd.instances {
                        if inst.object.contains("Wall") {
                            let hw = inst.sx * 16.0;
                            let hh = inst.sy * 16.0;
                            let p1 = to_radar(inst.x as f32 - hw, inst.y as f32 - hh);
                            let p2 = to_radar(inst.x as f32 + hw, inst.y as f32 + hh);
                            painter.rect_filled(Rect::from_min_max(p1, p2), 1.0, Color32::from_rgb(0, 185, 230));
                        } else if inst.object.contains("Arcade") || inst.object.contains("Stage") || inst.object.contains("Carousel") {
                            let rp = to_radar(inst.x as f32, inst.y as f32);
                            painter.circle_filled(rp, 2.5, Color32::from_rgb(255, 170, 50));
                        } else if inst.object.contains("Animatronic") || inst.object.contains("Escape") || inst.object.contains("Hatchet") {
                            let rp = to_radar(inst.x as f32, inst.y as f32);
                            painter.circle_filled(rp, 3.0, Color32::from_rgb(255, 215, 0));
                        }
                    }
                }

                let player_rp = to_radar(self.maze_player_x, self.maze_player_y);
                painter.circle_filled(player_rp, 4.5, Color32::from_rgb(0, 255, 180));
                painter.circle_stroke(player_rp, 5.5, Stroke::new(1.0_f32, Color32::WHITE));

                let cone_len = if self.radar_display_mode == 1 { 32.0_f32 } else { 22.0_f32 };
                let c_p1 = Pos2::new(player_rp.x + ((rad - 0.4).cos() * cone_len), player_rp.y + ((rad - 0.4).sin() * cone_len));
                let c_p2 = Pos2::new(player_rp.x + ((rad + 0.4).cos() * cone_len), player_rp.y + ((rad + 0.4).sin() * cone_len));
                painter.add(egui::Shape::convex_polygon(
                    vec![player_rp, c_p1, c_p2],
                    Color32::from_rgba_unmultiplied(255, 235, 90, 45),
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 235, 90, 160)),
                ));

                let radar_id = egui::Id::new("radar_click_drag");
                let radar_interact = ui.interact(radar_rect, radar_id, Sense::click_and_drag());
                if radar_interact.clicked() || radar_interact.dragged() {
                    if let Some(mp) = radar_interact.hover_pos() {
                        let nx = ((mp.x - radar_rect.min.x - 8.0) / (radar_w - 16.0)).clamp(0.0, 1.0);
                        let ny = ((mp.y - radar_rect.min.y - 24.0) / (radar_h - 32.0)).clamp(0.0, 1.0);
                        self.maze_player_x = min_x + (nx * (max_x - min_x));
                        self.maze_player_y = min_y + (ny * (max_y - min_y));
                    }
                }
            }

            // HUD Superior Izquierdo: Información de Sala y Zona
            let hud_top = Rect::from_min_size(Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0), Vec2::new(380.0, 52.0));
            painter.rect_filled(hud_top, 4.0, Color32::from_rgba_unmultiplied(10, 14, 20, 210));
            painter.rect_stroke(hud_top, 4.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 40)));
            painter.text(
                Pos2::new(hud_top.min.x + 8.0, hud_top.min.y + 6.0),
                egui::Align2::LEFT_TOP,
                format!("{} {}", ic::MAP_PIN, self.selected_salvage_location.display_name()),
                egui::FontId::proportional(13.0),
                Color32::from_rgb(255, 200, 80),
            );
            painter.text(
                Pos2::new(hud_top.min.x + 8.0, hud_top.min.y + 28.0),
                egui::Align2::LEFT_TOP,
                format!("Zona: {} | BSO: {}", zone_name, self.selected_salvage_location.soundtrack_name()),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(110, 220, 255),
            );

            // HUD Inferior: Coordenadas, Controles y Brújula Rápida
            let hud_bottom = Rect::from_min_size(Pos2::new(rect.min.x + 12.0, rect.max.y - 36.0), Vec2::new(640.0, 26.0));
            painter.rect_filled(hud_bottom, 4.0, Color32::from_rgba_unmultiplied(10, 14, 20, 210));
            
            let noclip_status = if self.noclip_mode { " [NOCLIP]" } else { "" };
            painter.text(
                Pos2::new(hud_bottom.min.x + 8.0, hud_bottom.center().y),
                egui::Align2::LEFT_CENTER,
                format!("[W][A][S][D]: Caminar | [Shift]: Correr | X: {:.0}, Y: {:.0} | Rumbo: {:.0}° ({}){}: {}", self.maze_player_x, self.maze_player_y, self.maze_player_yaw, self.move_speed_multiplier, noclip_status, zone_name),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(140, 215, 255),
            );

            // Banner central si está pausado
            if self.salvage_sim_paused {
                let pb = Rect::from_center_size(rect.center(), Vec2::new(260.0, 40.0));
                painter.rect_filled(pb, 6.0, Color32::from_rgba_unmultiplied(20, 25, 35, 230));
                painter.rect_stroke(pb, 6.0, Stroke::new(1.5_f32, Color32::from_rgb(255, 190, 60)));
                painter.text(
                    pb.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{} SIMULACIÓN PAUSADA", ic::PAUSE),
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(255, 200, 80),
                );
            }

            if resp.clicked() {
                self.salvage_flashlight = !self.salvage_flashlight;
            }

            // Panel Opcional Plegable para Animatrónico Custom
            ui.add_space(6.0);
            egui::CollapsingHeader::new(RichText::new("⚙️  Herramienta Opcional: Colocar Animatrónico en Coordenadas (X, Y)").size(12.5))
                .default_open(false)
                .show(ui, |ui| {
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
                });
        } else {
            // ==============================================================
            // MODO 2: DEBUGGER DE PLANO CENITAL 2D COMPLETO
            // ==============================================================
            ui.columns(2, |cols| {
                cols[0].heading(format!("{}  Debugger de Sala & Entidades", ic::BUG));
                cols[0].label("Inspección de las instancias reales extraídas del motor para las salas de laberinto:");
                cols[0].add_space(8.0);

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
                        }
                    }
                } else {
                    cols[0].label(RichText::new("Haz clic en cualquier entidad en el plano derecho para ver sus coordenadas.").color(Color32::GRAY));
                }

                cols[0].add_space(10.0);
                cols[0].separator();
                cols[0].add_space(6.0);

                cols[0].heading(format!("{} Posición del Jugador en el Plano", ic::COMPASS));
                cols[0].label(format!("Coordenadas Actuales: ({:.0}, {:.0}) | Rumbo: {:.0}°", self.maze_player_x, self.maze_player_y, self.maze_player_yaw));
                if cols[0].button("Fijar Jugador en Spawn Inicial").clicked() {
                    self.reset_maze_player_spawn();
                }

                // COLUMNA 2: PLANO CENITAL 2D
                cols[1].heading(format!("{}  Plano Cenital Completo ({})", ic::MAP_TRIFOLD, self.project.salvage_placement.target_room));
                cols[1].label("Visualización a escala de las instancias reales de GameMaker. Clic para inspeccionar coordenadas:");
                cols[1].add_space(6.0);

                let (rect, resp) = cols[1].allocate_exact_size(Vec2::new(540.0, 380.0), Sense::click_and_drag());
                let painter = cols[1].painter_at(rect);

                painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 15, 20));
                painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 75)));

                if let Some(r_data) = &active_room_data {
                    let to_screen = |gx: i32, gy: i32| -> Pos2 {
                        let nx = (gx as f32 - min_x) / (max_x - min_x).max(1.0);
                        let ny = (gy as f32 - min_y) / (max_y - min_y).max(1.0);
                        Pos2::new(
                            rect.min.x + 10.0 + (nx * (rect.width() - 20.0)),
                            rect.min.y + 10.0 + (ny * (rect.height() - 20.0)),
                        )
                    };

                    let mut clicked_inst_id = None;

                    for inst in &r_data.instances {
                        let is_wall = inst.object.contains("Wall");
                        let is_prop = inst.object.contains("Prop") || inst.object.contains("Decoration");
                        let is_spawn = inst.object.contains("Spawn");
                        let is_interactive = inst.object.contains("Interaction");

                        if is_wall && !self.dbg_filter_walls { continue; }
                        if is_prop && !self.dbg_filter_props { continue; }
                        if is_interactive && !self.dbg_filter_interactive { continue; }
                        if is_spawn && !self.dbg_filter_spawns { continue; }

                        let sp = to_screen(inst.x, inst.y);
                        let (color, radius) = if is_wall {
                            (Color32::from_rgb(100, 115, 140), 2.5)
                        } else if is_spawn {
                            (Color32::from_rgb(255, 90, 80), 5.0)
                        } else if is_interactive {
                            (Color32::from_rgb(60, 230, 110), 6.0)
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

                    // Marcador del Jugador en el plano 2D
                    let p_pos = to_screen(self.maze_player_x as i32, self.maze_player_y as i32);
                    painter.circle_filled(p_pos, 6.0, Color32::from_rgb(0, 255, 180));
                    painter.circle_stroke(p_pos, 7.5, Stroke::new(1.5_f32, Color32::WHITE));
                }
            });
        }
    }

}
