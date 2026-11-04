mod builder;
mod models;

use builder::ProjectBuilder;
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, RichText, Rounding, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use egui_phosphor::regular as ic;
use models::AnimatronicProject;
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
    Gallery,
    SpritesAndGuide,
    LiveAnimationPreview,
    GridAndMechanics,
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
    active_tab: ActiveTab,
    preview_state: PreviewState,
    sim_time: f32,
    status_message: String,
    status_is_error: bool,
    tex_cache: HashMap<PathBuf, TextureHandle>,
    _audio_stream: Option<OutputStream>,
    audio_sink: Option<Sink>,
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

        Self {
            project: AnimatronicProject::default(),
            active_tab: ActiveTab::Gallery,
            preview_state: PreviewState::IdleWithGlasses,
            sim_time: 0.0,
            status_message: "Estudio listo. Todos los subsistemas cargados.".to_string(),
            status_is_error: false,
            tex_cache: HashMap::new(),
            _audio_stream: stream,
            audio_sink: sink,
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

            // Las 4 Pestañas del Estudio
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Gallery,
                    RichText::new(format!("{}  1. Mod Gallery & Bio", ic::IDENTIFICATION_CARD)).size(14.0),
                );
                ui.add_space(6.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::SpritesAndGuide,
                    RichText::new(format!("{}  2. Sprites, Audios y Guía de Uso", ic::IMAGE)).size(14.0),
                );
                ui.add_space(6.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::LiveAnimationPreview,
                    RichText::new(format!("{}  3. Simulador de Animación en Vivo", ic::FILM_STRIP)).size(14.0),
                );
                ui.add_space(6.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::GridAndMechanics,
                    RichText::new(format!("{}  4. Orden de Tiles (Custom Night) & IA", ic::GRID_FOUR)).size(14.0),
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
                let btn = egui::Button::new(
                    RichText::new(format!("{}  Exportar Todo (Galería, Sprites 640x640 y GML)", ic::ROCKET_LAUNCH))
                        .size(14.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(205, 115, 25))
                .min_size(Vec2::new(340.0, 38.0));

                if ui.add(btn).clicked() {
                    match ProjectBuilder::build_all(&self.project) {
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
                ActiveTab::Gallery => self.ui_gallery_tab(ctx, ui),
                ActiveTab::SpritesAndGuide => self.ui_sprites_and_audio_tab(ctx, ui),
                ActiveTab::LiveAnimationPreview => self.ui_live_animation_tab(ctx, ui),
                ActiveTab::GridAndMechanics => self.ui_grid_and_mechanics_tab(ctx, ui),
            });
        });
    }
}

impl FrickEditorApp {
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

            let g_tex = self.project.accessory_glasses_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));
            Self::asset_card_row(
                &mut cols[0],
                "4. Gafas Clicables (s<ID>_Glasses)",
                "USO: DrawDepth=52. Recibe el clic exclusivo del ratón y sale disparada con gravedad.",
                &mut self.project.accessory_glasses_sprite,
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

            let k1 = self.project.sound_knock.clone();
            self.audio_card_row(&mut cols[1], "1. Golpe en Ventana (murray_knock.ogg)", "Suena al entrar al Estado 1", k1, 1);
            cols[1].add_space(6.0);

            let k2 = self.project.sound_voice_arrive.clone();
            self.audio_card_row(&mut cols[1], "2. Frase al Abrir Persiana (murray_amigable.ogg)", "Saludo al levantar la persiana izquierda", k2, 2);
            cols[1].add_space(6.0);

            let k3 = self.project.sound_defeat.clone();
            self.audio_card_row(&mut cols[1], "3. Caída Cómica de Alce (murray_knock2.ogg)", "Suena al flashearlo sin lentes", k3, 3);
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
        let glass_t = self.project.accessory_glasses_sprite.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 128));
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
                    let gx = center.x + (self.project.mechanics.glasses_offset_x as f32 - 52.0);
                    let gy = center.y - 8.0 + bob_head + (self.project.mechanics.glasses_offset_y as f32 - 48.0);
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
    // PESTAÑA 4: ORDEN DE TILES EN CUSTOM NIGHT Y AJUSTE DE IA
    // --------------------------------------------------------------
    fn ui_grid_and_mechanics_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let icon_tex = self.project.icon_48_path.clone().and_then(|p| self.get_or_load_tex(ctx, &p, 96));

        ui.columns(2, |cols| {
            // Columna Izquierda: Selector Visual de Orden en la Página de Mods (32 Slots = 8x4)
            cols[0].heading(format!("{}  Posición y Orden del Tile en la Página de Mods", ic::GRID_FOUR));
            cols[0].label("Haz clic en cualquier casilla para elegir en qué orden aparece el animatrónico en la Página 2 (grilla nativa 8x4):");
            cols[0].add_space(8.0);

            egui::Frame::group(cols[0].style())
                .fill(Color32::BLACK)
                .stroke(Stroke::new(1.5_f32, Color32::from_gray(160)))
                .inner_margin(10.0)
                .show(&mut cols[0], |ui| {
                    egui::Grid::new("custom_night_page2_grid")
                        .spacing([6.0, 8.0])
                        .show(ui, |ui| {
                            for slot in 0..32 {
                                let is_selected = self.project.mechanics.custom_night_slot_index == slot;
                                let (rect, resp) = ui.allocate_exact_size(Vec2::new(40.0, 48.0), Sense::click());
                                if resp.clicked() {
                                    self.project.mechanics.custom_night_slot_index = slot;
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
                                        let img_r = Rect::from_min_size(rect.min + Vec2::new(2.0, 2.0), Vec2::new(36.0, 36.0));
                                        p.image(tex.id(), img_r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                                    }
                                    p.rect_stroke(rect, 3.0, Stroke::new(2.0_f32, Color32::YELLOW));
                                } else {
                                    p.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        format!("#{}", slot + 1),
                                        egui::FontId::proportional(12.0),
                                        Color32::from_gray(130),
                                    );
                                }

                                if (slot + 1) % 8 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });

            let (tx, ty) = self.project.mechanics.slot_to_xy();
            cols[0].add_space(6.0);
            cols[0].label(
                RichText::new(format!(
                    "{} Casilla seleccionada: Slot #{}  ->  Coordenadas en GML: tile_x = {}, tile_y = {}",
                    ic::CHECK_CIRCLE,
                    self.project.mechanics.custom_night_slot_index + 1,
                    tx,
                    ty
                ))
                .color(Color32::from_rgb(90, 215, 130))
                .strong(),
            );

            // Columna Derecha: Parámetros de IA, Tiempos y Textos In-Game
            cols[1].heading(format!("{}  Parámetros de IA, 4 Layers Down y Textos", ic::SLIDERS_HORIZONTAL));
            cols[1].add_space(6.0);

            cols[1].add(
                egui::Slider::new(&mut self.project.mechanics.endless_default_ai, 1..=20)
                    .text("IA Automática en Modo Endless"),
            );
            cols[1].add(
                egui::Slider::new(&mut self.project.mechanics.four_layers_down_ai, 1..=20)
                    .text("IA Automática en Four Layers Down"),
            );
            cols[1].add(
                egui::Slider::new(&mut self.project.mechanics.attack_frames_ai20, 120..=600)
                    .text("Frames para Atacar en IA 20 (320 = 5.3s)"),
            );
            cols[1].add(
                egui::Slider::new(&mut self.project.mechanics.glasses_offset_x, 0..=120)
                    .text("Offset X de Gafas sobre la cabeza"),
            );
            cols[1].add(
                egui::Slider::new(&mut self.project.mechanics.glasses_offset_y, 0..=120)
                    .text("Offset Y de Gafas sobre la cabeza"),
            );

            cols[1].add_space(8.0);
            cols[1].label(RichText::new("Descripción emergente en Custom Night (Tooltip):").strong());
            cols[1].add(
                egui::TextEdit::multiline(&mut self.project.mechanics.custom_night_tooltip)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );

            cols[1].add_space(6.0);
            cols[1].label(RichText::new("Consejo en Pantalla de Muerte (Game Over Tip):").strong());
            cols[1].add(
                egui::TextEdit::multiline(&mut self.project.mechanics.gameover_tip)
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
                                    1 => self.project.sound_knock = Some(path),
                                    2 => self.project.sound_voice_arrive = Some(path),
                                    _ => self.project.sound_defeat = Some(path),
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
}
