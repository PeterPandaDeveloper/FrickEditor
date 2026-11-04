mod builder;
mod models;

use builder::Module1Builder;
use eframe::egui;
use models::AnimatronicProject;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([820.0, 640.0])
            .with_title("FrickEditor - Módulo 1: Galería, Sprites y Audios"),
        ..Default::default()
    };
    eframe::run_native(
        "FrickEditor",
        options,
        Box::new(|_cc| Ok(Box::new(FrickEditorApp::default()))),
    )
}

#[derive(PartialEq)]
enum ActiveTab {
    Gallery,
    Sprites,
    Audio,
}

struct FrickEditorApp {
    project: AnimatronicProject,
    active_tab: ActiveTab,
    status_message: String,
}

impl Default for FrickEditorApp {
    fn default() -> Self {
        Self {
            project: AnimatronicProject::default(),
            active_tab: ActiveTab::Gallery,
            status_message: "Listo. Configura la galería, sube los sprites/audios y presiona Construir.".to_string(),
        }
    }
}

impl eframe::App for FrickEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("🦌 FrickEditor — Módulo 1 (Asset & Gallery Studio)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("💾 Guardar Proyecto (.json)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("FrickEditor Project", &["json"])
                            .set_file_name("murray_project.json")
                            .save_file()
                        {
                            if let Ok(json) = serde_json::to_string_pretty(&self.project) {
                                let _ = std::fs::write(path, json);
                                self.status_message = "Proyecto guardado correctamente.".to_string();
                            }
                        }
                    }
                    if ui.button("📂 Cargar Proyecto").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("FrickEditor Project", &["json"])
                            .pick_file()
                        {
                            if let Ok(data) = std::fs::read_to_string(path) {
                                if let Ok(loaded) = serde_json::from_str::<AnimatronicProject>(&data) {
                                    self.project = loaded;
                                    self.status_message = "Proyecto cargado correctamente.".to_string();
                                }
                            }
                        }
                    }
                });
            });
            ui.add_space(4.0);
            ui.separator();

            // Barra de pestañas
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ActiveTab::Gallery, "🖼 1. Mod Gallery & Bio");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Sprites, "🎨 2. Sprites (Oficina y Jumpscare)");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Audio, "🔊 3. Audios (.ogg)");
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        [280.0, 36.0],
                        egui::Button::new("🚀 Exportar e Instalar Galería y Assets"),
                    )
                    .clicked()
                {
                    match Module1Builder::build_and_deploy(&self.project) {
                        Ok(msg) => self.status_message = msg,
                        Err(err) => self.status_message = format!("❌ Error: {}", err),
                    }
                }
                ui.label(&self.status_message);
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Configuración de rutas globales
                egui::CollapsingHeader::new("⚙ Rutas del Juego y Workspace")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("ID Interno (sin espacios):");
                            ui.text_edit_singleline(&mut self.project.internal_id);
                        });
                        Self::folder_picker_row(ui, "Carpeta del Juego (donde está data.win):", &mut self.project.game_dir);
                        Self::folder_picker_row(ui, "Carpeta de Trabajo (Workspace del Mod):", &mut self.project.workspace_dir);
                    });

                ui.separator();

                match self.active_tab {
                    ActiveTab::Gallery => self.render_gallery_tab(ui),
                    ActiveTab::Sprites => self.render_sprites_tab(ui),
                    ActiveTab::Audio => self.render_audio_tab(ui),
                }
            });
        });
    }
}

impl FrickEditorApp {
    fn render_gallery_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Personalización de Mod Gallery (addons/<ID>/extras_info.txt)");
        ui.label("Estos datos aparecerán en el menú Extras -> Mod Gallery del juego sin crashear.");
        ui.add_space(8.0);

        egui::Grid::new("gallery_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Nombre Completo (FULL_NAME):");
                ui.text_edit_singleline(&mut self.project.gallery_info.full_name);
                ui.end_row();

                ui.label("Año / Debut (DEBUT):");
                ui.text_edit_singleline(&mut self.project.gallery_info.debut);
                ui.end_row();

                ui.label("Crédito de Voz (VACredit):");
                ui.text_edit_singleline(&mut self.project.gallery_info.va_credit);
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.label("Descripción Lore / Mecánica (DESCRIPTION):");
        ui.add(egui::TextEdit::multiline(&mut self.project.gallery_info.description).desired_rows(4).desired_width(f32::INFINITY));

        ui.add_space(8.0);
        ui.label("Notas de Desarrollador (DEV_NOTES):");
        ui.add(egui::TextEdit::multiline(&mut self.project.gallery_info.dev_notes).desired_rows(4).desired_width(f32::INFINITY));

        ui.separator();
        ui.heading("Imágenes de Galería y Casilla de Custom Night");
        Self::file_picker_row(
            ui,
            "Portrait de Galería (se auto-ajusta a 500x500 px):",
            &mut self.project.portrait_500_path,
            &["png", "jpg"],
        );
        Self::file_picker_row(
            ui,
            "Icono de Casilla / Tile (se auto-ajusta a 48x48 px):",
            &mut self.project.icon_48_path,
            &["png"],
        );
        ui.checkbox(
            &mut self.project.auto_generate_gray_tile,
            "Generar automáticamente el Tile en escala de grises (sTile_<ID>_1.png) para AI = 0 / Death Coin",
        );
    }

    fn render_sprites_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sprites del Animatrónico en Oficina");
        ui.label("Selecciona los PNG transparentes para cada parte del animatrónico:");
        ui.add_space(8.0);

        Self::file_picker_row(ui, "Torso / Cuerpo (s<ID>_body):", &mut self.project.body_sprite, &["png"]);
        Self::file_picker_row(ui, "Cabeza Normal (s<ID>_head):", &mut self.project.head_normal_sprite, &["png"]);
        Self::file_picker_row(ui, "Cabeza Asustado sin gafas (s<ID>_head_Scare):", &mut self.project.head_scare_sprite, &["png"]);
        Self::file_picker_row(ui, "Gafas / Accesorio clicable (s<ID>_Glasses):", &mut self.project.accessory_glasses_sprite, &["png"]);
        Self::file_picker_row(ui, "Sprite Derrotado por Flash (s<ID>_Flash):", &mut self.project.flash_defeat_sprite, &["png"]);
        Self::file_picker_row(ui, "Cabeza Game Over Carmesí (s<ID>_head_Alt):", &mut self.project.head_gameover_alt, &["png"]);

        ui.separator();
        ui.heading("Fotogramas del Jumpscare (Auto-Composición 640x640)");
        ui.label("El programa los unirá automáticamente en lienzos limpios de 640x640 para evitar cortes:");
        Self::file_picker_row(ui, "Jumpscare Base / Cuerpo (Frame 0):", &mut self.project.jumpscare_frame_0, &["png"]);
        Self::file_picker_row(ui, "Jumpscare Cabeza 1 (Frame 1):", &mut self.project.jumpscare_frame_1, &["png"]);
        Self::file_picker_row(ui, "Jumpscare Cabeza 2 (Frame 2):", &mut self.project.jumpscare_frame_2, &["png"]);
    }

    fn render_audio_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Archivos de Sonido (.ogg)");
        ui.label("Se copiarán listos para reproducirse en el directorio del juego:");
        ui.add_space(8.0);

        Self::file_picker_row(ui, "1. Golpe en la ventana (murray_knock.ogg):", &mut self.project.sound_knock, &["ogg", "wav"]);
        Self::file_picker_row(ui, "2. Frase al abrir persiana (murray_amigable.ogg):", &mut self.project.sound_voice_arrive, &["ogg", "wav"]);
        Self::file_picker_row(ui, "3. Sonido cómico de alce al caer derrotado (murray_knock2.ogg):", &mut self.project.sound_defeat, &["ogg", "wav"]);
    }

    fn file_picker_row(ui: &mut egui::Ui, label: &str, target: &mut Option<PathBuf>, exts: &[&str]) {
        ui.horizontal(|ui| {
            ui.label(label);
            let current_text = target
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(Ninguno seleccionado)".to_string());
            ui.monospace(&current_text);
            if ui.button("Seleccionar...").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Archivos", exts).pick_file() {
                    *target = Some(path);
                }
            }
        });
    }

    fn folder_picker_row(ui: &mut egui::Ui, label: &str, target: &mut PathBuf) {
        ui.horizontal(|ui| {
            ui.label(label);
            let mut s = target.display().to_string();
            if ui.text_edit_singleline(&mut s).changed() {
                *target = PathBuf::from(s);
            }
            if ui.button("Explorar...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    *target = path;
                }
            }
        });
    }
}
