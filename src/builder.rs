use crate::models::AnimatronicProject;
use image::{imageops, DynamicImage, ImageBuffer, Rgba};
use std::fs;
use std::path::Path;

pub struct ProjectBuilder;

impl ProjectBuilder {
    pub fn build_all(proj: &AnimatronicProject) -> Result<String, String> {
        let id = proj.internal_id.trim();
        if id.is_empty() {
            return Err("El identificador interno (ID) no puede estar vacío.".to_string());
        }

        // 1. Crear addons/<ID> y purgar skins/<ID> (Invariante #6)
        let addon_dir = proj.game_dir.join("addons").join(id);
        fs::create_dir_all(&addon_dir).map_err(|e| e.to_string())?;

        let skin_dir = proj.game_dir.join("skins").join(id);
        if skin_dir.exists() {
            let _ = fs::remove_dir_all(&skin_dir);
        }
        let bad_render = addon_dir.join("FB_Murray_Moose.png");
        if bad_render.exists() {
            let _ = fs::remove_file(&bad_render);
        }

        // 2. Escribir extras_info.txt
        let extras_json = serde_json::to_string_pretty(&proj.gallery_info).map_err(|e| e.to_string())?;
        fs::write(addon_dir.join("extras_info.txt"), extras_json).map_err(|e| e.to_string())?;

        // 3. Procesar portrait.png (500x500)
        if let Some(portrait_src) = &proj.portrait_500_path {
            let img = image::open(portrait_src).map_err(|e| format!("Portrait inválido: {}", e))?;
            img.resize_exact(500, 500, imageops::FilterType::Lanczos3)
                .save(addon_dir.join("portrait.png"))
                .map_err(|e| e.to_string())?;
        }

        // 4. Carpeta de sprites para el Atlas
        let import_dir = proj.workspace_dir.join(format!("{}_Sprites_To_Import", id));
        fs::create_dir_all(&import_dir).map_err(|e| e.to_string())?;

        // 5. Procesar icon.png (48x48) + sTile_<ID>_0.png (Color) + sTile_<ID>_1.png (Gris)
        if let Some(icon_src) = &proj.icon_48_path {
            let img = image::open(icon_src).map_err(|e| format!("Icono inválido: {}", e))?;
            let icon_48 = img.resize_exact(48, 48, imageops::FilterType::Nearest);
            icon_48.save(addon_dir.join("icon.png")).map_err(|e| e.to_string())?;
            icon_48
                .save(import_dir.join(format!("sTile_{}_0.png", id)))
                .map_err(|e| e.to_string())?;

            if proj.auto_generate_gray_tile {
                let gray_img = DynamicImage::ImageLumaA8(icon_48.to_luma_alpha8()).to_rgba8();
                gray_img
                    .save(import_dir.join(format!("sTile_{}_1.png", id)))
                    .map_err(|e| e.to_string())?;
            }
        }

        // 6. Copiar Sprites de Oficina
        Self::copy_sprite(&proj.body_sprite, &import_dir, &format!("s{}_body_0.png", id))?;
        Self::copy_sprite(&proj.head_normal_sprite, &import_dir, &format!("s{}_head_0.png", id))?;
        Self::copy_sprite(&proj.head_scare_sprite, &import_dir, &format!("s{}_head_Scare_0.png", id))?;
        Self::copy_sprite(&proj.head_gameover_alt, &import_dir, &format!("s{}_head_Alt_0.png", id))?;
        Self::copy_sprite(&proj.accessory_glasses_sprite, &import_dir, &format!("s{}_Glasses_0.png", id))?;
        Self::copy_sprite(&proj.flash_defeat_sprite, &import_dir, &format!("s{}_Flash_0.png", id))?;

        // 7. Pre-componer Jumpscare (640x640)
        if let (Some(j0), Some(j1), Some(j2)) = (
            &proj.jumpscare_frame_0,
            &proj.jumpscare_frame_1,
            &proj.jumpscare_frame_2,
        ) {
            Self::precomposite_jumpscare(j0, j1, j2, &import_dir, id)?;
        }

        // 8. Copiar Audios (.ogg) a la carpeta del juego
        let id_low = id.to_lowercase();
        Self::copy_audio(&proj.sound_knock, &proj.game_dir, &format!("{}_knock.ogg", id_low))?;
        Self::copy_audio(&proj.sound_voice_arrive, &proj.game_dir, &format!("{}_amigable.ogg", id_low))?;
        Self::copy_audio(&proj.sound_defeat, &proj.game_dir, &format!("{}_knock2.ogg", id_low))?;

        // 9. Generar los scripts GML configurados con el Slot elegido y tiempos de IA
        let gml_dir = proj.workspace_dir.join(format!("{}_GML_Code", id));
        fs::create_dir_all(&gml_dir).map_err(|e| e.to_string())?;
        Self::generate_gml_files(proj, &gml_dir)?;

        let (tx, ty) = proj.mechanics.slot_to_xy();
        Ok(format!(
            "Exportación total lista: Galería + Sprites 640x640 + GML en Slot #{} (X={}, Y={}).",
            proj.mechanics.custom_night_slot_index + 1,
            tx,
            ty
        ))
    }

    fn generate_gml_files(proj: &AnimatronicProject, gml_dir: &Path) -> Result<(), String> {
        let id = proj.internal_id.trim();
        let (tile_x, tile_y) = proj.mechanics.slot_to_xy();

        // Generar Create_0 del Tile en la posición exacta del Slot elegido
        let cn_create = format!(
            "// Generado por FrickEditor Studio — Slot #{slot}\n\
             depth = -1000;\n\
             if (!variable_global_exists(\"{id}_AI\")) global.{id}_AI = 0;\n\
             if (!variable_global_exists(\"CustomNight_Page\")) global.CustomNight_Page = 0;\n\
             btn_page_x = 382;\n\
             btn_page_y = 332;\n\
             btn_page_w = 88;\n\
             btn_page_h = 24;\n\
             tile_x = {tx};\n\
             tile_y = {ty};\n\
             prev_freddy_ai = -1;\n",
            slot = proj.mechanics.custom_night_slot_index + 1,
            id = id,
            tx = tile_x,
            ty = tile_y
        );
        fs::write(
            gml_dir.join(format!("gml_Object_o{}_CustomNightTile_Create_0.gml", id)),
            cn_create,
        )
        .map_err(|e| e.to_string())?;

        // Generar Create_0 de la Oficina con los parámetros de IA y 4LD/Endless
        let bw_create = format!(
            "// Generado por FrickEditor Studio — Invariante #2 Blindada\n\
             global.{id}Death = false;\n\
             global.{id}_SavedTime = \"0:00.00\";\n\
             {id}Body = noone;\n\
             {id}Head = noone;\n\
             {id}Glasses = noone;\n\
             var ai = 0;\n\
             if (variable_global_exists(\"{id}_AI\")) ai = clamp(global.{id}_AI, 0, 20);\n\
             if (variable_global_exists(\"{id}_Disabled\") && global.{id}_Disabled) ai = 0;\n\
             if (ai == 0 && variable_global_exists(\"EndlessMode\") && global.EndlessMode == true) ai = {endless_ai};\n\
             else if (ai == 0 && variable_global_exists(\"FourLayersDown\") && global.FourLayersDown == true) ai = {fld_ai};\n\
             if (ai <= 0) {{ instance_destroy(); exit; }}\n\
             state = 0;\n\
             survival_frames = 0;\n\
             attack_timer = round(lerp({atk1}, {atk20}, ai / 20.0));\n\
             knock_timer = irandom_range(round(lerp(480, {kmin}, ai / 20.0)), round(lerp(900, {kmax}, ai / 20.0)));\n\
             has_glasses = true;\n\
             glasses_flying = false;\n\
             glasses_vx = 0;\n\
             glasses_vy = 0;\n\
             blind_fade = 0;\n\
             jumpscare_timer = 0;\n\
             jumpscare_frame = 0;\n\
             remove_cooldown = 0;\n\
             head_recoil_x = 0;\n\
             head_recoil_y = 0;\n",
            id = id,
            endless_ai = proj.mechanics.endless_default_ai,
            fld_ai = proj.mechanics.four_layers_down_ai,
            atk1 = proj.mechanics.attack_frames_ai1,
            atk20 = proj.mechanics.attack_frames_ai20,
            kmin = proj.mechanics.knock_min_frames_ai20,
            kmax = proj.mechanics.knock_max_frames_ai20,
        );
        fs::write(
            gml_dir.join(format!("gml_Object_o{}_BackWindow_Create_0.gml", id)),
            bw_create,
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn copy_sprite(src: &Option<PathBuf>, dst_dir: &Path, dst_name: &str) -> Result<(), String> {
        if let Some(path) = src {
            if path.exists() {
                fs::copy(path, dst_dir.join(dst_name)).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn copy_audio(src: &Option<PathBuf>, game_dir: &Path, dst_name: &str) -> Result<(), String> {
        if let Some(path) = src {
            if path.exists() {
                fs::copy(path, game_dir.join(dst_name)).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

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

        let configs = [
            (0, &head1, 0i64, 0i64),
            (1, &head2, 0i64, 10i64),
            (2, &head2, 0i64, -6i64),
        ];

        for (idx, head, dy_body, dy_head) in configs {
            let mut canvas: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(640, 640);
            imageops::overlay(&mut canvas, &body, 0, dy_body);
            imageops::overlay(&mut canvas, *head, 0, dy_head);

            let out_name = format!("s{}_Jumpscare_{}.png", id, idx);
            canvas
                .save(import_dir.join(out_name))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
