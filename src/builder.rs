use crate::models::AnimatronicProject;
use image::{imageops, DynamicImage, ImageBuffer, Rgba};
use std::fs;
use std::path::Path;

pub struct Module1Builder;

impl Module1Builder {
    pub fn build_and_deploy(proj: &AnimatronicProject) -> Result<String, String> {
        let id = proj.internal_id.trim();
        if id.is_empty() {
            return Err("El ID interno del animatrónico no puede estar vacío.".to_string());
        }

        // 1. Crear carpeta en addons/<ID> dentro del juego
        let addon_dir = proj.game_dir.join("addons").join(id);
        fs::create_dir_all(&addon_dir)
            .map_err(|e| format!("Error creando carpeta addons/{}: {}", id, e))?;

        // 2. INVARIANTE #6: Purgar cualquier carpeta skins/<ID> que cause crash al iniciar
        let skin_dir = proj.game_dir.join("skins").join(id);
        if skin_dir.exists() {
            let _ = fs::remove_dir_all(&skin_dir);
        }

        // 3. Escribir extras_info.txt en formato JSON
        let extras_json = serde_json::to_string_pretty(&proj.gallery_info)
            .map_err(|e| format!("Error serializando extras_info.txt: {}", e))?;
        fs::write(addon_dir.join("extras_info.txt"), extras_json)
            .map_err(|e| format!("Error guardando extras_info.txt: {}", e))?;

        // 4. Procesar y redimensionar portrait.png (500x500 exactos para evitar crash 0xc0000409)
        if let Some(portrait_src) = &proj.portrait_500_path {
            let img = image::open(portrait_src)
                .map_err(|e| format!("No se pudo abrir el Portrait: {}", e))?;
            let resized = img.resize_exact(500, 500, imageops::FilterType::Lanczos3);
            resized
                .save(addon_dir.join("portrait.png"))
                .map_err(|e| format!("Error guardando portrait.png: {}", e))?;
        }

        // 5. Preparar carpeta de sprites listos para importar al Atlas
        let import_dir = proj
            .workspace_dir
            .join(format!("{}_Sprites_To_Import", id));
        fs::create_dir_all(&import_dir)
            .map_err(|e| format!("Error creando carpeta de importación: {}", e))?;

        // 6. Procesar icon.png (48x48) + Generar sTile_<ID>_0.png (Color) y sTile_<ID>_1.png (Gris)
        if let Some(icon_src) = &proj.icon_48_path {
            let img = image::open(icon_src)
                .map_err(|e| format!("No se pudo abrir el Icono: {}", e))?;
            let icon_48 = img.resize_exact(48, 48, imageops::FilterType::Nearest);

            // Guardar icon.png en addons/<ID>/icon.png (elimina el cuadro blanco en Mod Gallery)
            icon_48
                .save(addon_dir.join("icon.png"))
                .map_err(|e| format!("Error guardando addons/icon.png: {}", e))?;

            // Guardar Frame 0 (Activo a color) para Custom Night
            let tile_0_name = format!("sTile_{}_0.png", id);
            icon_48
                .save(import_dir.join(&tile_0_name))
                .map_err(|e| format!("Error guardando {}: {}", tile_0_name, e))?;

            // INVARIANTE #5: Generar automáticamente Frame 1 (Escala de grises para AI=0 / Death Coin)
            if proj.auto_generate_gray_tile {
                let gray_img = DynamicImage::ImageLumaA8(icon_48.to_luma_alpha8()).to_rgba8();
                let tile_1_name = format!("sTile_{}_1.png", id);
                gray_img
                    .save(import_dir.join(&tile_1_name))
                    .map_err(|e| format!("Error guardando {}: {}", tile_1_name, e))?;
            }
        }

        // 7. Copiar Sprites de Oficina con nomenclatura estándar de GameMaker (_0.png)
        Self::copy_sprite(&proj.body_sprite, &import_dir, &format!("s{}_body_0.png", id))?;
        Self::copy_sprite(&proj.head_normal_sprite, &import_dir, &format!("s{}_head_0.png", id))?;
        Self::copy_sprite(&proj.head_scare_sprite, &import_dir, &format!("s{}_head_Scare_0.png", id))?;
        Self::copy_sprite(&proj.head_gameover_alt, &import_dir, &format!("s{}_head_Alt_0.png", id))?;
        Self::copy_sprite(&proj.accessory_glasses_sprite, &import_dir, &format!("s{}_Glasses_0.png", id))?;
        Self::copy_sprite(&proj.flash_defeat_sprite, &import_dir, &format!("s{}_Flash_0.png", id))?;

        // 8. Pre-componer Jumpscare en 640x640 (Evita el bug de los 4 cuadrantes partidos)
        if let (Some(j0), Some(j1), Some(j2)) = (
            &proj.jumpscare_frame_0,
            &proj.jumpscare_frame_1,
            &proj.jumpscare_frame_2,
        ) {
            Self::precomposite_jumpscare(j0, j1, j2, &import_dir, id)?;
        }

        // 9. Copiar Audios (.ogg) al directorio del juego
        Self::copy_audio(&proj.sound_knock, &proj.game_dir, &format!("{}_knock.ogg", id.to_lowercase()))?;
        Self::copy_audio(&proj.sound_voice_arrive, &proj.game_dir, &format!("{}_amigable.ogg", id.to_lowercase()))?;
        Self::copy_audio(&proj.sound_defeat, &proj.game_dir, &format!("{}_knock2.ogg", id.to_lowercase()))?;

        Ok(format!(
            "¡Módulo 1 completado con éxito!\n\n\
             - Galería creada en: {}\n\
             - Sprites procesados y pre-compuestos en: {}\n\
             - Audios desplegados en la raíz del juego.",
            addon_dir.display(),
            import_dir.display()
        ))
    }

    fn copy_sprite(src: &Option<std::path::PathBuf>, dst_dir: &Path, dst_name: &str) -> Result<(), String> {
        if let Some(path) = src {
            if path.exists() {
                fs::copy(path, dst_dir.join(dst_name))
                    .map_err(|e| format!("Error copiando {}: {}", dst_name, e))?;
            }
        }
        Ok(())
    }

    fn copy_audio(src: &Option<std::path::PathBuf>, game_dir: &Path, dst_name: &str) -> Result<(), String> {
        if let Some(path) = src {
            if path.exists() {
                fs::copy(path, game_dir.join(dst_name))
                    .map_err(|e| format!("Error copiando audio {}: {}", dst_name, e))?;
            }
        }
        Ok(())
    }

    /// Une el torso (j0) con las cabezas (j1, j2) en 3 lienzos limpios de 640x640
    fn precomposite_jumpscare(
        j0_path: &Path,
        j1_path: &Path,
        j2_path: &Path,
        import_dir: &Path,
        id: &str,
    ) -> Result<(), String> {
        let body = image::open(j0_path).map_err(|e| e.to_string())?;
        let head1 = image::open(j1_path).map_err(|e| e.to_string())?;
        let head2 = image::open(j2_path).map_err(|e| e.to_string())?;

        let configs = [(0, &head1, 0i64, 0i64), (1, &head2, 0i64, 10i64), (2, &head2, 0i64, -6i64)];

        for (idx, head, dy_body, dy_head) in configs {
            let mut canvas: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(640, 640);
            imageops::overlay(&mut canvas, &body, 0, dy_body);
            imageops::overlay(&mut canvas, *head, 0, dy_head);

            let out_name = format!("s{}_Jumpscare_{}.png", id, idx);
            canvas
                .save(import_dir.join(out_name))
                .map_err(|e| format!("Error guardando jumpscare frame {}: {}", idx, e))?;
        }
        Ok(())
    }
}
