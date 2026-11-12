use crate::models::{AnimatronicArchetype, AnimatronicProject, CountermeasureType};
use image::{imageops, DynamicImage, ImageBuffer, Rgba};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

pub struct IdeBuilder;

impl IdeBuilder {
    /// Genera los 13 scripts GML sin fugas de memoria, adaptados al Arquetipo real y seguros para Multi-Mod
    pub fn generate_template_gml_scripts(proj: &AnimatronicProject) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        let id = proj.internal_id.trim();
        let id_low = id.to_lowercase();
        let (cx, cy) = proj.behavior.archetype.default_coordinates(proj.behavior.use_right_side);
        let (tile_x, tile_y) = proj.custom_night.slot_to_xy();
        let b = &proj.behavior;

        // ==========================================================
        // 1. CleanUp_0 (Invariante #2 + Destrucción de Audio Streams)
        // ==========================================================
        let cleanup_code = format!(
            "// FrickEditor IDE — CleanUp_0 Auditado (Invariante #2 + Anti Memory-Leak)\n\
             if (variable_instance_exists(id, \"{id}Body\") && {id}Body != noone && instance_exists({id}Body)) {{\n\
                 instance_destroy({id}Body);\n\
                 {id}Body = noone;\n\
             }}\n\
             if (variable_instance_exists(id, \"{id}Head\") && {id}Head != noone && instance_exists({id}Head)) {{\n\
                 instance_destroy({id}Head);\n\
                 {id}Head = noone;\n\
             }}\n\
             if (variable_instance_exists(id, \"{id}Glasses\") && {id}Glasses != noone && instance_exists({id}Glasses)) {{\n\
                 instance_destroy({id}Glasses);\n\
                 {id}Glasses = noone;\n\
             }}\n\
             if (variable_instance_exists(id, \"snd_stream_knock\") && snd_stream_knock != -1) {{\n\
                 audio_destroy_stream(snd_stream_knock);\n\
                 snd_stream_knock = -1;\n\
             }}\n\
             if (variable_instance_exists(id, \"snd_stream_voice\") && snd_stream_voice != -1) {{\n\
                 audio_destroy_stream(snd_stream_voice);\n\
                 snd_stream_voice = -1;\n\
             }}\n\
             if (variable_instance_exists(id, \"snd_stream_defeat\") && snd_stream_defeat != -1) {{\n\
                 audio_destroy_stream(snd_stream_defeat);\n\
                 snd_stream_defeat = -1;\n\
             }}\n\
             if (variable_instance_exists(id, \"snd_stream_jump\") && snd_stream_jump != -1) {{\n\
                 audio_destroy_stream(snd_stream_jump);\n\
                 snd_stream_jump = -1;\n\
             }}\n"
        );
        map.insert(format!("gml_Object_o{}_BackWindow_CleanUp_0.gml", id), cleanup_code);

        // ==========================================================
        // 2. Create_0 (Inicialización Absoluta en Línea 1 + Streams)
        // ==========================================================
        let fld_branch = if b.appear_in_four_layers_down {
            format!("else if (ai == 0 && variable_global_exists(\"FourLayersDown\") && global.FourLayersDown == true) ai = {};", b.four_layers_down_ai)
        } else {
            String::new()
        };
        let endless_branch = if b.appear_in_endless_mode {
            format!("if (ai == 0 && variable_global_exists(\"EndlessMode\") && global.EndlessMode == true) ai = {};", b.endless_mode_ai)
        } else {
            String::new()
        };

        let create_code = format!(
            "// FrickEditor IDE — Create_0 Auditado ({arch})\n\
             // INVARIANTE #2: Inicializar absolutamente todo antes de cualquier exit\n\
             global.{id}Death = false;\n\
             global.{id}_SavedTime = \"0:00.00\";\n\
             {id}Body = noone;\n\
             {id}Head = noone;\n\
             {id}Glasses = noone;\n\
             snd_stream_knock = -1;\n\
             snd_stream_voice = -1;\n\
             snd_stream_defeat = -1;\n\
             snd_stream_jump = -1;\n\
             state = 0;\n\
             survival_frames = 0;\n\
             attack_timer = 600;\n\
             knock_timer = 600;\n\
             has_glasses = {has_acc};\n\
             glasses_flying = false;\n\
             glasses_vx = 0;\n\
             glasses_vy = 0;\n\
             blind_fade = 0;\n\
             jumpscare_timer = 0;\n\
             jumpscare_frame = 0;\n\
             remove_cooldown = 0;\n\
             head_recoil_x = 0;\n\
             head_recoil_y = 0;\n\
             \n\
             var ai = 0;\n\
             if (variable_global_exists(\"{id}_AI\")) ai = clamp(global.{id}_AI, 0, 20);\n\
             if (variable_global_exists(\"{id}_Disabled\") && global.{id}_Disabled) ai = 0;\n\
             {endless_branch}\n\
             {fld_branch}\n\
             if (ai <= 0) {{\n\
                 instance_destroy();\n\
                 exit;\n\
             }}\n\
             \n\
             // Pre-cargar streams de audio una sola vez en memoria\n\
             if (file_exists(\"{id_low}_knock.ogg\")) snd_stream_knock = audio_create_stream(\"{id_low}_knock.ogg\");\n\
             if (file_exists(\"{id_low}_amigable.ogg\")) snd_stream_voice = audio_create_stream(\"{id_low}_amigable.ogg\");\n\
             if (file_exists(\"{id_low}_knock2.ogg\")) snd_stream_defeat = audio_create_stream(\"{id_low}_knock2.ogg\");\n\
             if (file_exists(\"{id_low}_jumpscare.ogg\")) snd_stream_jump = audio_create_stream(\"{id_low}_jumpscare.ogg\");\n\
             \n\
             attack_timer = round(lerp({atk1}, {atk20}, ai / 20.0));\n\
             knock_timer = irandom_range(round(lerp({kmin1}, {kmin20}, ai / 20.0)), round(lerp({kmax1}, {kmax20}, ai / 20.0)));\n{camera_gml}",
            arch = b.archetype.display_name(),
            id = id,
            id_low = id_low,
            endless_branch = endless_branch,
            fld_branch = fld_branch,
            atk1 = b.attack_window_frames_ai1,
            atk20 = b.attack_window_frames_ai20,
            kmin1 = b.move_min_frames_ai1,
            kmin20 = b.move_min_frames_ai20,
            kmax1 = b.move_max_frames_ai1,
            kmax20 = b.move_max_frames_ai20,
            has_acc = if b.countermeasure == CountermeasureType::RemoveAccessoryThenFlash { "true" } else { "false" },
            camera_gml = if !proj.advanced.camera_route.is_empty() {
                let cam_ids: Vec<String> = proj.advanced.camera_route.iter().map(|n| n.camera_id.to_string()).collect();
                format!(
                    "             // Ruteo de Cámaras (Room 33: MainLocationMap)\n\
                     camera_route = [{}];\n\
                     camera_step = 0;\n\
                     camera_timer = 180;\n",
                    cam_ids.join(", ")
                )
            } else {
                String::new()
            }
        );
        map.insert(format!("gml_Object_o{}_BackWindow_Create_0.gml", id), create_code);

        // ==========================================================
        // 3. Draw_0 y 4. Draw_64
        // ==========================================================
        map.insert(
            format!("gml_Object_o{}_BackWindow_Draw_0.gml", id),
            "// Renderizado en oficina mediante instancias oOfficeSprite\n".to_string(),
        );

        let draw64_code = format!(
            "// FrickEditor IDE — Draw_64 (Jumpscare 640x640 sin costuras)\n\
             if (state == 4) {{\n\
                 var s_jump = asset_get_index(\"s{id}_Jumpscare\");\n\
                 if (s_jump != -1) {{\n\
                     var gw = display_get_gui_width();\n\
                     var gh = display_get_gui_height();\n\
                     var lunge_t = clamp(jumpscare_timer / 24.0, 0.0, 1.0);\n\
                     var lunge_curve = power(lunge_t, 0.45);\n\
                     var base_gui = max(gw / 640.0, gh / 360.0);\n\
                     var zoom_factor = lerp(0.52, 1.25, lunge_curve);\n\
                     var scale = base_gui * zoom_factor;\n\
                     var sw = 640.0 * scale;\n\
                     var sh = 640.0 * scale;\n\
                     var shake_intensity = lerp(3.0, 12.0, lunge_curve) * (gw / 640.0);\n\
                     var shake_x = irandom_range(-round(shake_intensity), round(shake_intensity));\n\
                     var shake_y = irandom_range(-round(shake_intensity), round(shake_intensity));\n\
                     var sub_img = (floor(jumpscare_timer / 3) % 3);\n\
                     if (jumpscare_timer > 68) sub_img = 2;\n\
                     var draw_x = ((gw - sw) / 2.0) + shake_x;\n\
                     var draw_y = ((gh - sh) / 2.0) + (40.0 * scale) + shake_y;\n\
                     draw_sprite_ext(s_jump, sub_img, draw_x, draw_y, scale, scale, 0, c_white, 1.0);\n\
                 }}\n\
             }}\n"
        );
        map.insert(format!("gml_Object_o{}_BackWindow_Draw_64.gml", id), draw64_code);

        // ==========================================================
        // 5. Step_0 (Adaptado al Arquetipo Real y Condición de Defensa)
        // ==========================================================
        let blocker_check = match b.archetype {
            AnimatronicArchetype::HallwayDoorway => {
                "if (instance_exists(my_door) && !my_door.Open) blocked = true;"
            }
            AnimatronicArchetype::BackWindowBlinds => {
                "if (instance_exists(my_blind) && !my_blind.Open) blocked = true;"
            }
            _ => "",
        };

        let defeat_condition_gml = match b.countermeasure {
            CountermeasureType::RemoveAccessoryThenFlash => {
                "(!has_glasses && !blocked && flashing && remove_cooldown <= 0 && hover_window)"
            }
            CountermeasureType::DirectFlashlight => "(!blocked && flashing && hover_window)",
            CountermeasureType::DoNotFlashLight => "(attack_timer <= 60 && !flashing)",
            CountermeasureType::CloseDoorOrBlind => "(blocked && attack_timer <= 120)",
            CountermeasureType::WearFreddyMask => "(mask_on && attack_timer <= 180)",
            CountermeasureType::AudioLureOrShock => "(keyboard_check_pressed(vk_Control) || (variable_global_exists(\"Shocked\") && global.Shocked))",
            CountermeasureType::SolveBackConsoleTask => "(hover_window && mouse_check_button_pressed(mb_left))",
        };

        let queue_check = if b.smart_queue_sun_moon && b.archetype == AnimatronicArchetype::BackWindowBlinds {
            "var window_busy = false;\n\
             if (instance_exists(oSunMoon)) {\n\
                 with (oSunMoon) {\n\
                     if (variable_instance_exists(id, \"AI\") && AI > 0 && variable_instance_exists(id, \"SunActive\") && SunActive) window_busy = true;\n\
                 }\n\
             }\n\
             if (instance_exists(oOfficeSunPoster)) {\n\
                 with (oOfficeSunPoster) {\n\
                     if (image_index == 1 && instance_exists(other.my_blind) && !other.my_blind.Open) window_busy = true;\n\
                 }\n\
             }"
        } else {
            "var window_busy = false;"
        };

        let open_blind_action = if b.opens_blind_on_arrival && b.archetype == AnimatronicArchetype::BackWindowBlinds {
            "if (instance_exists(my_blind)) my_blind.Open = true;"
        } else {
            ""
        };

        let step_code = format!(
            "// FrickEditor IDE — Step_0 Auditado ({arch_name})\n\
             survival_frames++;\n\
             var tot_sec = survival_frames / 60.0;\n\
             var s_min = floor(tot_sec / 60.0);\n\
             var s_sec = floor(tot_sec) mod 60;\n\
             var s_dec = floor((tot_sec - floor(tot_sec)) * 100.0);\n\
             global.{id}_SavedTime = string(s_min) + \":\" + (s_sec < 10 ? \"0\" : \"\") + string(s_sec) + \".\" + (s_dec < 10 ? \"0\" : \"\") + string(s_dec);\n\
             \n\
             var cx = {cx};\n\
             var cy = {cy};\n\
             var my_blind = instance_nearest(cx, cy, oWindowBlinds);\n\
             var my_door = instance_nearest(cx, cy, oDoor);\n\
             \n\
             var deathcoined = false;\n\
             if (variable_global_exists(\"{id}_DeathCoined\") && global.{id}_DeathCoined) deathcoined = true;\n\
             if (variable_global_exists(\"{id}_Disabled\") && global.{id}_Disabled) deathcoined = true;\n\
             if (variable_global_exists(\"DeathCoin_Target\") && global.DeathCoin_Target == \"{id}\") deathcoined = true;\n\
             if ({allow_dc} && state == 1 && mouse_check_button_pressed(mb_left)) {{\n\
                 if (variable_global_exists(\"HeldItem\") && (global.HeldItem == \"DeathCoin\" || global.HeldItem == \"Death Coin\" || global.HeldItem == \"oShopItem_DeathCoin\")) deathcoined = true;\n\
             }}\n\
             if (deathcoined) {{\n\
                 if ({id}Body != noone && instance_exists({id}Body)) {{ instance_destroy({id}Body); {id}Body = noone; }}\n\
                 if ({id}Head != noone && instance_exists({id}Head)) {{ instance_destroy({id}Head); {id}Head = noone; }}\n\
                 if ({id}Glasses != noone && instance_exists({id}Glasses)) {{ instance_destroy({id}Glasses); {id}Glasses = noone; }}\n\
                 glasses_flying = false;\n\
                 var snd_dc = asset_get_index(\"sfxDeathCoin\");\n\
                 if (snd_dc == -1) snd_dc = asset_get_index(\"sfxIgnitedFreddy_Hit1\");\n\
                 if (snd_dc != -1) audio_play_sound(snd_dc, 10, false);\n\
                 global.{id}_DeathCoined = true;\n\
                 global.{id}_Disabled = true;\n\
                 instance_destroy();\n\
                 exit;\n\
             }}\n\
             \n\
             if (glasses_flying && {id}Glasses != noone && instance_exists({id}Glasses)) {{\n\
                 {id}Glasses.x += glasses_vx;\n\
                 {id}Glasses.y += glasses_vy;\n\
                 glasses_vy += 1.2;\n\
                 if ({id}Glasses.y > 1350) {{\n\
                     instance_destroy({id}Glasses);\n\
                     {id}Glasses = noone;\n\
                     glasses_flying = false;\n\
                 }}\n\
             }}\n\
             \n\
             if (state == 0) {{\n\
                 if ({id}Body != noone && instance_exists({id}Body)) {{ instance_destroy({id}Body); {id}Body = noone; }}\n\
                 if ({id}Head != noone && instance_exists({id}Head)) {{ instance_destroy({id}Head); {id}Head = noone; }}\n\
                 if ({id}Glasses != noone && instance_exists({id}Glasses)) {{ instance_destroy({id}Glasses); {id}Glasses = noone; }}\n\
                 glasses_flying = false;\n\
                 knock_timer--;\n\
                 if (knock_timer <= 0) {{\n\
                     {queue_check}\n\
                     if (window_busy) {{\n\
                         knock_timer = 30;\n\
                     }} else {{\n\
                         state = 1;\n\
                         var ai = 20;\n\
                         if (variable_global_exists(\"{id}_AI\")) ai = clamp(global.{id}_AI, 0, 20);\n\
                         attack_timer = round(lerp({atk1}, {atk20}, ai / 20.0));\n\
                         has_glasses = {has_acc};\n\
                         remove_cooldown = 0;\n\
                         head_recoil_x = 0;\n\
                         head_recoil_y = 0;\n\
                         {open_blind_action}\n\
                         var spr_body = asset_get_index(\"s{id}_body\");\n\
                         var spr_head = asset_get_index(\"s{id}_head\");\n\
                         var spr_glasses = asset_get_index(\"s{id}_Glasses\");\n\
                         {id}Body = instance_create_layer(cx + ({bx}), cy + ({by}), \"Behind_Wall\", oOfficeSprite);\n\
                         if (spr_body != -1) {id}Body.sprite_index = spr_body;\n\
                         {id}Body.DrawDepth = 48;\n\
                         {id}Head = instance_create_layer(cx + ({hx}), cy + ({hy}), \"Behind_Wall\", oOfficeSprite);\n\
                         if (spr_head != -1) {id}Head.sprite_index = spr_head;\n\
                         {id}Head.DrawDepth = 50;\n\
                         if (has_glasses) {{\n\
                             {id}Glasses = instance_create_layer(cx + ({hx}) + ({gx}), cy + ({hy}) + ({gy}), \"Behind_Wall\", oOfficeSprite);\n\
                             if (spr_glasses != -1) {id}Glasses.sprite_index = spr_glasses;\n\
                             {id}Glasses.DrawDepth = 52;\n\
                         }}\n\
                         if (snd_stream_knock != -1) {{\n\
                             audio_play_sound(snd_stream_knock, 10, false);\n\
                         }} else {{\n\
                             var snd_k = asset_get_index(\"sfxKnock\");\n\
                             if (snd_k != -1) audio_play_sound(snd_k, 10, false);\n\
                         }}\n\
                         if (snd_stream_voice != -1) {{\n\
                             audio_play_sound(snd_stream_voice, 11, false);\n\
                         }}\n\
                     }}\n\
                 }}\n\
             }}\n\
             \n\
             if (state == 1) {{\n\
                 var blocked = false;\n\
                 {blocker_check}\n\
                 if (!blocked || !{stalls_blind}) attack_timer--;\n\
                 if (remove_cooldown > 0) remove_cooldown--;\n\
                 head_recoil_x = lerp(head_recoil_x, 0, 0.15);\n\
                 head_recoil_y = lerp(head_recoil_y, 0, 0.15);\n\
                 var torso_bob = sin(current_time * 0.003) * 2.5;\n\
                 if ({id}Body != noone && instance_exists({id}Body)) {id}Body.y = (cy + ({by})) + torso_bob;\n\
                 if ({id}Head != noone && instance_exists({id}Head)) {{\n\
                     var head_bob = sin(current_time * 0.004) * 3.0;\n\
                     var jitter_x = has_glasses ? 0 : random_range(-2.2, 2.2);\n\
                     var jitter_y = has_glasses ? 0 : random_range(-2.2, 2.2);\n\
                     {id}Head.x = (cx + ({hx})) + head_recoil_x + jitter_x;\n\
                     {id}Head.y = (cy + ({hy})) + head_bob + head_recoil_y + jitter_y;\n\
                     if (has_glasses && {id}Glasses != noone && instance_exists({id}Glasses)) {{\n\
                         {id}Glasses.x = {id}Head.x + ({gx});\n\
                         {id}Glasses.y = {id}Head.y + ({gy});\n\
                     }}\n\
                 }}\n\
                 var mx = mouse_x;\n\
                 var my = mouse_y;\n\
                 if (instance_exists(o3DMouse)) {{ mx = o3DMouse.x; my = o3DMouse.y; }}\n\
                 var hover_glasses_only = false;\n\
                 if (has_glasses && {id}Glasses != noone && instance_exists({id}Glasses)) {{\n\
                     if (mx >= {id}Glasses.x - 8 && mx <= {id}Glasses.x + {gw} && my >= {id}Glasses.y - 8 && my <= {id}Glasses.y + {gh}) {{\n\
                         hover_glasses_only = true;\n\
                     }}\n\
                 }}\n\
                 var hover_window = (point_distance(cx, cy + 60, mx, my) < 175);\n\
                 var flashing = (keyboard_check(ord(\"F\")) || mouse_check_button(mb_right) || (variable_global_exists(\"Flashlight\") && global.Flashlight));\n\
                 var mask_on = (variable_global_exists(\"Mask\") && global.Mask);\n\
                 if (has_glasses && !blocked && hover_glasses_only && mouse_check_button_pressed(mb_left)) {{\n\
                     has_glasses = false;\n\
                     glasses_flying = true;\n\
                     glasses_vx = choose(-6.5, 6.5);\n\
                     glasses_vy = -8.5;\n\
                     remove_cooldown = 12;\n\
                     head_recoil_x = (glasses_vx > 0 ? 16 : -16);\n\
                     head_recoil_y = -8;\n\
                     var spr_scare = asset_get_index(\"s{id}_head_Scare\");\n\
                     if (spr_scare != -1 && {id}Head != noone && instance_exists({id}Head)) {id}Head.sprite_index = spr_scare;\n\
                     var snd_c = asset_get_index(\"sfxClick\");\n\
                     if (snd_c != -1) audio_play_sound(snd_c, 10, false);\n\
                 }}\n\
                 if ({defeat_cond}) {{\n\
                     state = 2;\n\
                     blind_fade = 55;\n\
                     if ({id}Body != noone && instance_exists({id}Body)) {{ instance_destroy({id}Body); {id}Body = noone; }}\n\
                     if ({id}Glasses != noone && instance_exists({id}Glasses)) {{ instance_destroy({id}Glasses); {id}Glasses = noone; }}\n\
                     glasses_flying = false;\n\
                     var spr_flash = asset_get_index(\"s{id}_Flash\");\n\
                     if ({id}Head != noone && instance_exists({id}Head)) {{\n\
                         if (spr_flash != -1) {id}Head.sprite_index = spr_flash;\n\
                         {id}Head.x = cx - 94;\n\
                         {id}Head.y = cy;\n\
                         {id}Head.DrawDepth = 54;\n\
                     }}\n\
                     if (snd_stream_defeat != -1) {{\n\
                         audio_play_sound(snd_stream_defeat, 12, false);\n\
                     }}\n\
                     var scr_tok = asset_get_index(\"gml_Script_add_tokens\");\n\
                     if (scr_tok != -1 && {tok} > 0) script_execute(scr_tok, {tok});\n\
                 }}\n\
                 if (attack_timer <= 0 && !blocked) {{\n\
                     state = 4;\n\
                     jumpscare_timer = 0;\n\
                     jumpscare_frame = 0;\n\
                     glasses_flying = false;\n\
                     if ({id}Body != noone && instance_exists({id}Body)) {{ instance_destroy({id}Body); {id}Body = noone; }}\n\
                     if ({id}Head != noone && instance_exists({id}Head)) {{ instance_destroy({id}Head); {id}Head = noone; }}\n\
                     if ({id}Glasses != noone && instance_exists({id}Glasses)) {{ instance_destroy({id}Glasses); {id}Glasses = noone; }}\n\
                     if (snd_stream_jump != -1) {{\n\
                         audio_play_sound(snd_stream_jump, 100, false);\n\
                     }} else {{\n\
                         var scream = asset_get_index(\"sfxJumpscareNormal\");\n\
                         if (scream == -1) scream = asset_get_index(\"sfxWindowScare\");\n\
                         if (scream != -1) audio_play_sound(scream, 10, false);\n\
                     }}\n\
                 }}\n\
             }}\n\
             \n\
             if (state == 2) {{\n\
                 blind_fade--;\n\
                 if ({id}Head != noone && instance_exists({id}Head)) {{\n\
                     {id}Head.x = (cx - 94) + random_range(-3, 3);\n\
                     if (blind_fade < 35) {id}Head.y += 4.5;\n\
                 }}\n\
                 if (blind_fade <= 0) {{\n\
                     state = 0;\n\
                     var ai = 20;\n\
                     if (variable_global_exists(\"{id}_AI\")) ai = clamp(global.{id}_AI, 0, 20);\n\
                     knock_timer = irandom_range(round(lerp(1800, 600, ai / 20.0)), round(lerp(2800, 1000, ai / 20.0)));\n\
                     if ({id}Head != noone && instance_exists({id}Head)) {{ instance_destroy({id}Head); {id}Head = noone; }}\n\
                 }}\n\
             }}\n\
             \n\
             if (state == 4) {{\n\
                 jumpscare_timer++;\n\
                 jumpscare_frame = (floor(jumpscare_timer / 4) % 3);\n\
                 if (jumpscare_timer >= 85) {{\n\
                     global.{id}Death = true;\n\
                     global.Jumpscare = -1;\n\
                     global.TimeLasted = global.{id}_SavedTime;\n\
                     var r_go = asset_get_index(\"GameOver\");\n\
                     if (r_go != -1) room_goto(r_go);\n\
                     else game_restart();\n\
                 }}\n\
             }}\n",
            arch_name = b.archetype.display_name(),
            id = id,
            cx = cx,
            cy = cy,
            allow_dc = if b.allow_death_coin { "true" } else { "false" },
            queue_check = queue_check,
            atk1 = b.attack_window_frames_ai1,
            atk20 = b.attack_window_frames_ai20,
            has_acc = if b.countermeasure == CountermeasureType::RemoveAccessoryThenFlash { "true" } else { "false" },
            open_blind_action = open_blind_action,
            bx = b.body_offset_x,
            by = b.body_offset_y,
            hx = b.head_offset_x,
            hy = b.head_offset_y,
            gx = b.accessory_offset_x,
            gy = b.accessory_offset_y,
            gw = b.accessory_hitbox_w,
            gh = b.accessory_hitbox_h,
            blocker_check = blocker_check,
            defeat_cond = defeat_condition_gml,
            stalls_blind = if b.stalls_when_blocked { "true" } else { "false" },
            tok = b.token_reward_on_defeat,
        );
        map.insert(format!("gml_Object_o{}_BackWindow_Step_0.gml", id), step_code);

        // ==========================================================
        // 6. Custom Night Tile (Multi-Mod Seguro: Sin doble toggle ni sobreescritura de fondo)
        // ==========================================================
        let is_slot_zero = proj.custom_night.slot_index == 0;
        let tile_depth = if is_slot_zero { -1000 } else { -1005 };

        let cn_create = format!(
            "depth = {depth};\n\
             if (!variable_global_exists(\"{id}_AI\")) global.{id}_AI = 0;\n\
             if (!variable_global_exists(\"CustomNight_Page\")) global.CustomNight_Page = 0;\n\
             if (!variable_global_exists(\"CN_PageSwitchFrame\")) global.CN_PageSwitchFrame = -1;\n\
             btn_page_x = 382;\n\
             btn_page_y = 332;\n\
             btn_page_w = 88;\n\
             btn_page_h = 24;\n\
             tile_x = {tx};\n\
             tile_y = {ty};\n",
            depth = tile_depth,
            id = id,
            tx = tile_x,
            ty = tile_y
        );
        map.insert(format!("gml_Object_o{}_CustomNightTile_Create_0.gml", id), cn_create);

        let cn_step = format!(
            "depth = {depth};\n\
             if (!variable_global_exists(\"{id}_AI\")) global.{id}_AI = 0;\n\
             if (!variable_global_exists(\"CustomNight_Page\")) global.CustomNight_Page = 0;\n\
             with (oCustomNightTile) {{ visible = (global.CustomNight_Page == 0); }}\n\
             var snd_click = asset_get_index(\"sfxClick\");\n\
             if (snd_click == -1) snd_click = asset_get_index(\"snd_Button\");\n\
             var snd_btn = asset_get_index(\"snd_Button\");\n\
             if (mouse_check_button_pressed(mb_left)) {{\n\
                 // Guardia anti-colisión Multi-Mod: solo cambia de página una vez por clic\n\
                 if (mouse_x >= btn_page_x && mouse_x <= btn_page_x + btn_page_w && mouse_y >= btn_page_y && mouse_y <= btn_page_y + btn_page_h) {{\n\
                     if (global.CN_PageSwitchFrame != current_time) {{\n\
                         global.CN_PageSwitchFrame = current_time;\n\
                         global.CustomNight_Page = 1 - global.CustomNight_Page;\n\
                         if (snd_click != -1) audio_play_sound(snd_click, 10, false);\n\
                         with (oCustomNightTile) {{ visible = (global.CustomNight_Page == 0); }}\n\
                     }}\n\
                 }} else if (global.CustomNight_Page == 1) {{\n\
                     if (mouse_x >= tile_x && mouse_x <= tile_x + 32 && mouse_y >= tile_y + 48 && mouse_y <= tile_y + 66) {{\n\
                         if (mouse_x < tile_x + 16) {{ if (global.{id}_AI > 0) {{ global.{id}_AI--; if (snd_click != -1) audio_play_sound(snd_click, 0, false); }} }}\n\
                         else {{ if (global.{id}_AI < 20) {{ global.{id}_AI++; if (snd_click != -1) audio_play_sound(snd_click, 0, false); }} }}\n\
                     }}\n\
                 }}\n\
                 if (mouse_y >= 43 && mouse_y <= 75) {{\n\
                     if (mouse_x >= 480 && mouse_x <= 543) global.{id}_AI = 0;\n\
                     else if (mouse_x >= 544 && mouse_x <= 608) global.{id}_AI = min(20, global.{id}_AI + 1);\n\
                 }} else if (mouse_y >= 75 && mouse_y <= 107) {{\n\
                     if (mouse_x >= 480 && mouse_x <= 543) global.{id}_AI = min(20, global.{id}_AI + 5);\n\
                     else if (mouse_x >= 544 && mouse_x <= 608) global.{id}_AI = 20;\n\
                 }}\n\
             }}\n\
             if (global.CustomNight_Page == 1 && mouse_check_button_pressed(mb_right)) {{\n\
                 if (mouse_x >= tile_x && mouse_x <= tile_x + 48 && mouse_y >= tile_y && mouse_y <= tile_y + 66) {{\n\
                     global.{id}_AI = (global.{id}_AI > 0) ? 0 : 20;\n\
                     if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);\n\
                 }}\n\
             }}\n\
             if (global.CustomNight_Page == 1 && mouse_x >= tile_x && mouse_x <= tile_x + 48 && mouse_y >= tile_y && mouse_y <= tile_y + 66) {{\n\
                 if (mouse_wheel_up() && global.{id}_AI < 20) {{ global.{id}_AI++; if (snd_click != -1) audio_play_sound(snd_click, 0, false); }}\n\
                 else if (mouse_wheel_down() && global.{id}_AI > 0) {{ global.{id}_AI--; if (snd_click != -1) audio_play_sound(snd_click, 0, false); }}\n\
             }}\n",
            depth = tile_depth,
            id = id
        );
        map.insert(format!("gml_Object_o{}_CustomNightTile_Step_0.gml", id), cn_step);

        let escaped_tooltip = proj.custom_night.cursor_tooltip.replace('"', "\\\"");
        let bg_clear_gml = if is_slot_zero {
            "draw_set_color(c_black);\n    draw_rectangle(28, 0, 475, 360, false);"
        } else {
            "// Fondo limpiado por el Slot #0"
        };

        let cn_draw = format!(
            "var fnt = asset_get_index(\"fnt_Small\");\n\
             if (fnt == -1) fnt = asset_get_index(\"fnt_Dialogue\");\n\
             if (fnt != -1) draw_set_font(fnt);\n\
             if (global.CustomNight_Page == 1) {{\n\
                 {bg_clear}\n\
                 var spr_base = asset_get_index(\"sTileBase\");\n\
                 if (spr_base != -1) draw_sprite(spr_base, 0, tile_x, tile_y);\n\
                 var spr_tile = asset_get_index(\"sTile_{id}\");\n\
                 if (spr_tile != -1) {{\n\
                     var is_dc = (variable_global_exists(\"{id}_DeathCoined\") && global.{id}_DeathCoined);\n\
                     if (global.{id}_AI > 0 && !is_dc) draw_sprite(spr_tile, 0, tile_x, tile_y);\n\
                     else if (sprite_get_number(spr_tile) > 1) draw_sprite(spr_tile, 1, tile_x, tile_y);\n\
                     else draw_sprite_ext(spr_tile, 0, tile_x, tile_y, 1, 1, 0, c_gray, 1.0);\n\
                 }}\n\
                 var spr_arrows = asset_get_index(\"sCustomNightArrows\");\n\
                 if (spr_arrows != -1) draw_sprite(spr_arrows, 0, tile_x, tile_y + 48);\n\
                 var spr_counter = asset_get_index(\"sCustomNightCounter\");\n\
                 if (spr_counter != -1) draw_sprite(spr_counter, global.{id}_AI, tile_x + 32, tile_y + 48);\n\
                 var show_desc = (!variable_global_exists(\"ShowDescriptions\") || global.ShowDescriptions);\n\
                 if (show_desc && mouse_x >= tile_x && mouse_x <= tile_x + 48 && mouse_y >= tile_y && mouse_y <= tile_y + 66) {{\n\
                     var tip_txt = \"{tooltip}\";\n\
                     var tw = 224; var th = 68;\n\
                     var tx = clamp(mouse_x + 12, 32, 470 - tw);\n\
                     var ty = clamp(mouse_y + 12, 12, 350 - th);\n\
                     draw_set_color(make_color_rgb(198, 198, 198));\n\
                     draw_rectangle(tx, ty, tx + tw, ty + th, false);\n\
                     draw_set_color(c_black);\n\
                     draw_rectangle(tx - 1, ty - 1, tx + tw + 1, ty + th + 1, true);\n\
                     draw_set_halign(fa_left); draw_set_valign(fa_top);\n\
                     draw_text_ext(tx + 6, ty + 6, tip_txt, 13, tw - 12);\n\
                 }}\n\
             }}\n\
             var hov_btn = (mouse_x >= btn_page_x && mouse_x <= btn_page_x + btn_page_w && mouse_y >= btn_page_y && mouse_y <= btn_page_y + btn_page_h);\n\
             draw_set_color(hov_btn ? make_color_rgb(215, 215, 215) : make_color_rgb(192, 192, 192));\n\
             draw_rectangle(btn_page_x, btn_page_y, btn_page_x + btn_page_w, btn_page_y + btn_page_h, false);\n\
             draw_set_color(c_black);\n\
             draw_rectangle(btn_page_x - 1, btn_page_y - 1, btn_page_x + btn_page_w + 1, btn_page_y + btn_page_h + 1, true);\n\
             var cy_mid = btn_page_y + (btn_page_h div 2);\n\
             draw_set_valign(fa_middle);\n\
             if (global.CustomNight_Page == 0) {{\n\
                 draw_set_halign(fa_left);\n\
                 draw_text(btn_page_x + 12, cy_mid, \"PAGE 2\");\n\
                 var ax = btn_page_x + btn_page_w - 18;\n\
                 draw_triangle(ax, cy_mid - 5, ax, cy_mid + 5, ax + 6, cy_mid, false);\n\
             }} else {{\n\
                 var ax = btn_page_x + 12;\n\
                 draw_triangle(ax + 6, cy_mid - 5, ax + 6, cy_mid + 5, ax, cy_mid, false);\n\
                 draw_set_halign(fa_left);\n\
                 draw_text(btn_page_x + 24, cy_mid, \"PAGE 1\");\n\
             }}\n\
             draw_set_halign(fa_left); draw_set_valign(fa_top); draw_set_color(c_white);\n",
            bg_clear = bg_clear_gml,
            id = id,
            tooltip = escaped_tooltip
        );
        map.insert(format!("gml_Object_o{}_CustomNightTile_Draw_0.gml", id), cn_draw);

        // ==========================================================
        // 7. DeathScreen (Draw_0 y Step_0)
        // ==========================================================
        let escaped_go_tip = proj.custom_night.gameover_tip.replace('"', "\\\"");
        let death_sync = format!(
            "if (room == asset_get_index(\"GameOver\")) {{\n\
                 if (variable_global_exists(\"{id}Death\") && global.{id}Death) {{\n\
                     if (variable_global_exists(\"{id}_SavedTime\")) global.TimeLasted = global.{id}_SavedTime;\n\
                     with (oGameOver) {{\n\
                         Tip = \"{tip}\";\n\
                         if (variable_instance_exists(id, \"TimeLasted\") && variable_global_exists(\"{id}_SavedTime\")) {{\n\
                             TimeLasted = global.{id}_SavedTime;\n\
                         }}\n\
                     }}\n\
                 }}\n\
             }}\n",
            id = id,
            tip = escaped_go_tip
        );
        map.insert(format!("gml_Object_o{}_DeathScreen_Draw_0.gml", id), death_sync.clone());
        map.insert(format!("gml_Object_o{}_DeathScreen_Step_0.gml", id), death_sync);

        // ==========================================================
        // 8. Menú de Pausa — Review Animatronics
        // ==========================================================
        let rev_create = "depth = -2500;\nreview_page = 0;\nbtn_rev_x = 524;\nbtn_rev_y = 322;\nbtn_rev_w = 96;\nbtn_rev_h = 24;\n".to_string();
        map.insert(format!("gml_Object_o{}_ReviewHandler_Create_0.gml", id), rev_create);

        let rev_step = "depth = -2500;\n\
             var in_pause_review = false;\n\
             if (instance_exists(oPauser)) {\n\
                 with (oPauser) {\n\
                     if (variable_instance_exists(id, \"Paused\") && Paused) in_pause_review = true;\n\
                     if (variable_instance_exists(id, \"Reviewing\") && Reviewing) in_pause_review = true;\n\
                 }\n\
             }\n\
             if (!in_pause_review) { review_page = 0; exit; }\n\
             if (mouse_check_button_pressed(mb_left)) {\n\
                 if (mouse_x >= btn_rev_x && mouse_x <= btn_rev_x + btn_rev_w && mouse_y >= btn_rev_y && mouse_y <= btn_rev_y + btn_rev_h) {\n\
                     review_page = 1 - review_page;\n\
                     var snd_c = asset_get_index(\"sfxClick\");\n\
                     if (snd_c != -1) audio_play_sound(snd_c, 10, false);\n\
                 }\n\
             }\n"
        .to_string();
        map.insert(format!("gml_Object_o{}_ReviewHandler_Step_0.gml", id), rev_step);

        let escaped_full_name = proj.gallery_info.full_name.replace('"', "\\\"");
        let escaped_loc = proj.custom_night.pause_review_location.replace('"', "\\\"");
        let escaped_rev_desc = proj.custom_night.pause_review_description.replace('"', "\\\"").replace('\n', "\\n");
        let rev_draw = format!(
            "var in_pause_review = false;\n\
             if (instance_exists(oPauser)) {{\n\
                 with (oPauser) {{\n\
                     if (variable_instance_exists(id, \"Paused\") && Paused) in_pause_review = true;\n\
                     if (variable_instance_exists(id, \"Reviewing\") && Reviewing) in_pause_review = true;\n\
                 }}\n\
             }}\n\
             if (!in_pause_review) exit;\n\
             var fnt = asset_get_index(\"fnt_Small\");\n\
             if (fnt != -1) draw_set_font(fnt);\n\
             if (review_page == 1) {{\n\
                 var px = 80; var py = 48; var pw = 480; var ph = 256;\n\
                 draw_set_color(make_color_rgb(198, 198, 198));\n\
                 draw_rectangle(px, py, px + pw, py + ph, false);\n\
                 draw_set_color(c_black);\n\
                 draw_rectangle(px - 1, py - 1, px + pw + 1, py + th + 1, true);\n\
                 var spr_tile = asset_get_index(\"sTile_{id}\");\n\
                 if (spr_tile != -1) draw_sprite(spr_tile, 0, px + 16, py + 16);\n\
                 draw_set_halign(fa_left); draw_set_valign(fa_top);\n\
                 draw_set_color(make_color_rgb(0, 0, 128));\n\
                 draw_text(px + 74, py + 16, \"{full_name}\");\n\
                 draw_set_color(make_color_rgb(60, 60, 60));\n\
                 draw_text(px + 74, py + 34, \"Location: {loc}\");\n\
                 draw_set_color(c_black);\n\
                 draw_text_ext(px + 16, py + 74, \"{rev_desc}\", 13, pw - 32);\n\
             }}\n\
             draw_set_color(make_color_rgb(192, 192, 192));\n\
             draw_rectangle(btn_rev_x, btn_rev_y, btn_rev_x + btn_rev_w, btn_rev_y + btn_rev_h, false);\n\
             draw_set_color(c_black);\n\
             draw_rectangle(btn_rev_x - 1, btn_rev_y - 1, btn_rev_x + btn_rev_w + 1, btn_rev_y + btn_rev_h + 1, true);\n\
             draw_set_halign(fa_center); draw_set_valign(fa_middle);\n\
             draw_text(btn_rev_x + (btn_rev_w div 2), btn_rev_y + (btn_rev_h div 2), review_page == 0 ? \"MOD REVIEW >\" : \"< BACK\");\n\
             draw_set_halign(fa_left); draw_set_valign(fa_top); draw_set_color(c_white);\n",
            id = id,
            full_name = escaped_full_name,
            loc = escaped_loc,
            rev_desc = escaped_rev_desc
        );
        map.insert(format!("gml_Object_o{}_ReviewHandler_Draw_0.gml", id), rev_draw);

        map
    }

    pub fn get_effective_gml_scripts(proj: &AnimatronicProject) -> BTreeMap<String, String> {
        let mut map = Self::generate_template_gml_scripts(proj);
        for (k, v) in &proj.behavior.custom_gml_overrides {
            if map.contains_key(k) {
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }

    pub fn find_next_csx_version(utmt_scripts_dir: &Path, id: &str) -> u32 {
        let mut max_ver = 26u32;
        if let Ok(entries) = fs::read_dir(utmt_scripts_dir) {
            let prefix = format!("Instalar_{}_V", id);
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if let Some(rest) = fname.strip_prefix(&prefix) {
                    if let Some(num_str) = rest.strip_suffix(".csx") {
                        if let Ok(v) = num_str.parse::<u32>() {
                            if v > max_ver {
                                max_ver = v;
                            }
                        }
                    }
                }
            }
        }
        max_ver + 1
    }

    pub fn build_full_ide_project(proj: &AnimatronicProject) -> Result<String, String> {
        let id = proj.internal_id.trim();
        if id.is_empty() {
            return Err("El ID interno del animatrónico no puede estar vacío.".to_string());
        }

        Self::build_phase1_visuals(proj)?;

        let gml_dir = proj.workspace_dir.join(format!("{}_GML_Code", id));
        fs::create_dir_all(&gml_dir).map_err(|e| e.to_string())?;
        let gml_map = Self::get_effective_gml_scripts(proj);
        for (filename, content) in &gml_map {
            fs::write(gml_dir.join(filename), content).map_err(|e| e.to_string())?;
        }

        fs::create_dir_all(&proj.utmt_scripts_dir).map_err(|e| e.to_string())?;
        let next_v = Self::find_next_csx_version(&proj.utmt_scripts_dir, id);
        let csx_name = format!("Instalar_{}_V{}.csx", id, next_v);
        let csx_path = proj.utmt_scripts_dir.join(&csx_name);

        let csx_code = Self::generate_csx_installer_code(proj, next_v);
        fs::write(&csx_path, csx_code).map_err(|e| e.to_string())?;

        Ok(format!(
            "¡Auditoría & Compilación 100% Exitosa! {} GMLs y '{}' (con Packer, Salas y GameOver) listos.",
            gml_map.len(),
            csx_name
        ))
    }

    /// Compila y exporta todos los animatrónicos activos seleccionados en el ModManager,
    /// generando sus carpetas de sprites, GMLs y el script maestro `master_injector.csx`.
    pub fn build_all_active_mods(
        mods: &[AnimatronicProject],
        game_dir: &Path,
        utmt_scripts_dir: &Path,
        headless: bool,
    ) -> Result<PathBuf, String> {
        if mods.is_empty() {
            return Err("No hay animatrónicos activos para construir.".to_string());
        }

        for proj in mods {
            let id = proj.internal_id.trim();
            if id.is_empty() {
                continue;
            }
            Self::build_phase1_visuals(proj)?;

            let gml_dir = proj.workspace_dir.join(format!("{}_GML_Code", id));
            fs::create_dir_all(&gml_dir).map_err(|e| e.to_string())?;
            let gml_map = Self::get_effective_gml_scripts(proj);
            for (filename, content) in &gml_map {
                fs::write(gml_dir.join(filename), content).map_err(|e| e.to_string())?;
            }
        }

        fs::create_dir_all(utmt_scripts_dir).map_err(|e| e.to_string())?;
        let master_csx_code = Self::generate_master_injector_csx(mods, game_dir, headless);
        let master_csx_path = utmt_scripts_dir.join("master_injector.csx");
        fs::write(&master_csx_path, master_csx_code).map_err(|e| e.to_string())?;

        Ok(master_csx_path)
    }

    /// Genera el script C# `master_injector.csx` combinando todos los animatrónicos activos
    pub fn generate_master_injector_csx(
        mods: &[AnimatronicProject],
        game_dir: &Path,
        headless: bool,
    ) -> String {
        let mut mods_code = String::new();
        let mut go_create_branches = Vec::new();
        let mut go_draw_branches = Vec::new();

        for proj in mods {
            let id = proj.internal_id.trim();
            if id.is_empty() {
                continue;
            }
            let import_dir = proj.workspace_dir.join(format!("{}_Sprites_To_Import", id));
            let gml_dir = proj.workspace_dir.join(format!("{}_GML_Code", id));
            let go_tip = proj.custom_night.gameover_tip.replace('"', "\\\"");

            go_create_branches.push(format!(
                "if (variable_global_exists(\"{id}Death\") && global.{id}Death) {{ Tip = \"{go_tip}\"; if (variable_global_exists(\"{id}_SavedTime\")) global.TimeLasted = global.{id}_SavedTime; }}"
            ));

            go_draw_branches.push(format!(
                "if (variable_global_exists(\"{id}Death\") && global.{id}Death) {{ var spr_alt = asset_get_index(\"s{id}_head_Alt\"); if (spr_alt == -1) spr_alt = asset_get_index(\"s{id}_head\"); if (spr_alt != -1) {{ var sc = 3.2; var sw = sprite_get_width(spr_alt) * sc; var sh = sprite_get_height(spr_alt) * sc; draw_sprite_ext(spr_alt, 0, (640 - sw) / 2, ((360 - sh) / 2) - 10, sc, sc, 0, make_color_rgb(175, 20, 20), 0.9); }} }}"
            ));

            mods_code.push_str(&format!(
                r#"
// ==========================================================
// Inyección Modular para Animatrónico: {id}
// ==========================================================
string importFolder_{id} = @"{import_dir}";
string codeDir_{id} = @"{gml_dir}";

if (Directory.Exists(importFolder_{id}))
{{
    Regex sprFrameRegex = new(@"^(.+?)(?:_(\d+))$", RegexOptions.Compiled);
    string packDir = Path.Join(ExePath, "Packager");
    Directory.CreateDirectory(packDir);
    string outName = Path.Join(packDir, "atlas_{id}_master.txt");

    Packer packer = new Packer();
    packer.Process(importFolder_{id}, "*.png", 2048, 2, false);
    packer.SaveAtlasses(outName);

    int lastTextPage = Data.EmbeddedTextures.Count - 1;
    int lastTextPageItem = Data.TexturePageItems.Count - 1;
    string prefix = Path.Join(Path.GetDirectoryName(outName), Path.GetFileNameWithoutExtension(outName));
    int atlasCount = 0;

    foreach (Atlas atlas in packer.Atlasses)
    {{
        string atlasName = $"{{prefix}}{{atlasCount:000}}.png";
        using MagickImage atlasImage = TextureWorker.ReadBGRAImageFromFile(atlasName);

        UndertaleEmbeddedTexture texture = new();
        texture.Name = new UndertaleString($"Texture {{++lastTextPage}}");
        texture.TextureData.Image = GMImage.FromMagickImage(atlasImage).ConvertToPng();
        Data.EmbeddedTextures.Add(texture);

        foreach (Node n in atlas.Nodes)
        {{
            if (n.Texture != null)
            {{
                UndertaleTexturePageItem texturePageItem = new();
                texturePageItem.Name = new UndertaleString($"PageItem {{++lastTextPageItem}}");
                texturePageItem.SourceX = (ushort)n.Bounds.X;
                texturePageItem.SourceY = (ushort)n.Bounds.Y;
                texturePageItem.SourceWidth = (ushort)n.Bounds.Width;
                texturePageItem.SourceHeight = (ushort)n.Bounds.Height;
                texturePageItem.TargetX = 0;
                texturePageItem.TargetY = 0;
                texturePageItem.TargetWidth = (ushort)n.Bounds.Width;
                texturePageItem.TargetHeight = (ushort)n.Bounds.Height;
                texturePageItem.BoundingWidth = (ushort)n.Texture.BoundingWidth;
                texturePageItem.BoundingHeight = (ushort)n.Texture.BoundingHeight;
                texturePageItem.TexturePage = texture;
                Data.TexturePageItems.Add(texturePageItem);

                string stripped = Path.GetFileNameWithoutExtension(n.Texture.Source);
                string spriteName = stripped;
                int frame = 0;
                var spriteParts = sprFrameRegex.Match(stripped);
                if (spriteParts.Success)
                {{
                    spriteName = spriteParts.Groups[1].Value;
                    int.TryParse(spriteParts.Groups[2].Value, out frame);
                }}

                UndertaleSprite.TextureEntry texentry = new() {{ Texture = texturePageItem }};
                UndertaleSprite sprite = Data.Sprites.ByName(spriteName);
                if (sprite is null)
                {{
                    UndertaleSprite newSprite = new();
                    newSprite.Name = Data.Strings.MakeString(spriteName);
                    newSprite.Width = (uint)n.Texture.BoundingWidth;
                    newSprite.Height = (uint)n.Texture.BoundingHeight;
                    newSprite.MarginLeft = 0;
                    newSprite.MarginRight = n.Bounds.Width - 1;
                    newSprite.MarginTop = 0;
                    newSprite.MarginBottom = n.Bounds.Height - 1;
                    newSprite.OriginX = 0;
                    newSprite.OriginY = 0;
                    for (int f = 0; f < frame; f++) newSprite.Textures.Add(null);
                    newSprite.Textures.Add(texentry);
                    Data.Sprites.Add(newSprite);
                }}
                else
                {{
                    sprite.Width = (uint)n.Texture.BoundingWidth;
                    sprite.Height = (uint)n.Texture.BoundingHeight;
                    sprite.OriginX = 0;
                    sprite.OriginY = 0;
                    while (frame >= sprite.Textures.Count) sprite.Textures.Add(texentry);
                    sprite.Textures[frame] = texentry;
                }}
            }}
        }}
        atlasCount++;
    }}
}}

UndertaleGameObject objAnim_{id} = Data.GameObjects.ByName("o{id}_BackWindow");
if (objAnim_{id} == null) {{ objAnim_{id} = new() {{ Name = Data.Strings.MakeString("o{id}_BackWindow"), Visible = true }}; Data.GameObjects.Add(objAnim_{id}); }}

UndertaleGameObject objDeath_{id} = Data.GameObjects.ByName("o{id}_DeathScreen");
if (objDeath_{id} == null) {{ objDeath_{id} = new() {{ Name = Data.Strings.MakeString("o{id}_DeathScreen"), Visible = true, Persistent = true }}; Data.GameObjects.Add(objDeath_{id}); }}

UndertaleGameObject objCNTile_{id} = Data.GameObjects.ByName("o{id}_CustomNightTile");
if (objCNTile_{id} == null) {{ objCNTile_{id} = new() {{ Name = Data.Strings.MakeString("o{id}_CustomNightTile"), Visible = true }}; Data.GameObjects.Add(objCNTile_{id}); }}

UndertaleGameObject objReview_{id} = Data.GameObjects.ByName("o{id}_ReviewHandler");
if (objReview_{id} == null) {{ objReview_{id} = new() {{ Name = Data.Strings.MakeString("o{id}_ReviewHandler"), Visible = true }}; Data.GameObjects.Add(objReview_{id}); }}

importGroup.QueueReplace(objAnim_{id}.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_BackWindow_Create_0.gml")));
importGroup.QueueReplace(objAnim_{id}.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_BackWindow_Step_0.gml")));
importGroup.QueueReplace(objAnim_{id}.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_BackWindow_Draw_0.gml")));
importGroup.QueueReplace(objAnim_{id}.EventHandlerFor(EventType.Draw, EventSubtypeDraw.DrawGUI, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_BackWindow_Draw_64.gml")));
importGroup.QueueReplace(objAnim_{id}.EventHandlerFor(EventType.CleanUp, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_BackWindow_CleanUp_0.gml")));

importGroup.QueueReplace(objDeath_{id}.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_DeathScreen_Draw_0.gml")));
importGroup.QueueReplace(objDeath_{id}.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_DeathScreen_Step_0.gml")));

importGroup.QueueReplace(objCNTile_{id}.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_CustomNightTile_Create_0.gml")));
importGroup.QueueReplace(objCNTile_{id}.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_CustomNightTile_Step_0.gml")));
importGroup.QueueReplace(objCNTile_{id}.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_CustomNightTile_Draw_0.gml")));

importGroup.QueueReplace(objReview_{id}.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_ReviewHandler_Create_0.gml")));
importGroup.QueueReplace(objReview_{id}.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_ReviewHandler_Step_0.gml")));
importGroup.QueueReplace(objReview_{id}.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir_{id}, "gml_Object_o{id}_ReviewHandler_Draw_0.gml")));

AddInstanceToRoom("Office", objAnim_{id}, 0, 0);
AddInstanceToRoom("Office", objDeath_{id}, 0, 0);
AddInstanceToRoom("Office", objReview_{id}, 0, 0);
AddInstanceToRoom("Fredbear_Office", objAnim_{id}, 0, 0);
AddInstanceToRoom("Fredbear_Office", objReview_{id}, 0, 0);
AddInstanceToRoom("GameOver", objDeath_{id}, 0, 0);
AddInstanceToRoom("CustomNightMenu", objCNTile_{id}, 0, 0);
AddInstanceToRoom("EndlessSelection", objCNTile_{id}, 0, 0);
AddInstanceToRoom("EndlessShopSelection_FinalNight", objCNTile_{id}, 0, 0);
{props_code}
"#,
                id = id,
                import_dir = import_dir.display(),
                gml_dir = gml_dir.display(),
                props_code = {
                    let mut code = String::new();
                    if !proj.advanced.maze_entities.is_empty() {
                        let target_room = &proj.advanced.target_maze_room;
                        let clean_room = if target_room.contains("SalvageResults") {
                            "SalvageResults"
                        } else if target_room.contains("SalvageEnding") {
                            "SalvageEnding"
                        } else {
                            target_room.as_str()
                        };
                        code.push_str(&format!("\n// Inyección de Props 3D / Laberinto en {clean_room} para {id}\n"));
                        for (idx, ent) in proj.advanced.maze_entities.iter().enumerate() {
                            let obj_name = ent.prop_type.gml_object_name();
                            code.push_str(&format!(
                                "UndertaleGameObject prop_{id}_{idx} = Data.GameObjects.ByName(\"{obj_name}\");\n\
                                 if (prop_{id}_{idx} != null) AddInstanceToRoom(\"{clean_room}\", prop_{id}_{idx}, {x}, {y});\n",
                                id = id,
                                idx = idx,
                                obj_name = obj_name,
                                clean_room = clean_room,
                                x = ent.x,
                                y = ent.y
                            ));
                        }
                    }
                    code
                }
            ));
        }

        let chained_create = if go_create_branches.is_empty() {
            "Tip = \"You passed out.\";".to_string()
        } else {
            format!("{} else {{ Tip = \"You passed out.\"; }}", go_create_branches.join(" else "))
        };

        let chained_draw = if go_draw_branches.is_empty() {
            "if (global.Jumpscare == -1)".to_string()
        } else {
            format!("{} else if (global.Jumpscare == -1)", go_draw_branches.join(" else "))
        };

        let post_injection_msg = if headless {
            r#"Console.WriteLine("¡INYECCIÓN MASTER COMPLETADA CON ÉXITO!");"#.to_string()
        } else {
            format!(
                r#"ScriptMessage("¡Inyección Master de {} animatrónico(s) completada! data.win guardado.");"#,
                mods.len()
            )
        };

        format!(
            r#"// ==============================================================
// Generado automáticamente por FrickEditor IDE — master_injector.csx
// Multi-Mod Master Injector (Total de animatrónicos activos: {num_mods})
// ==============================================================
using System;
using System.IO;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using UndertaleModLib;
using UndertaleModLib.Util;
using UndertaleModLib.Models;
using ImageMagick;

EnsureDataLoaded();

string gameDir = @"{game_dir}";

void AddInstanceToRoom(string roomName, UndertaleGameObject targetObj, int targetX, int targetY)
{{
    UndertaleRoom rm = Data.Rooms.ByName(roomName);
    if (rm == null) return;
    foreach (var l in rm.Layers)
    {{
        if (l.LayerType == UndertaleRoom.LayerType.Instances && l.InstancesData != null)
        {{
            if (!l.InstancesData.Instances.Any(i => i.ObjectDefinition == targetObj))
            {{
                l.InstancesData.Instances.Add(new UndertaleRoom.GameObject() {{ InstanceID = Data.GeneralInfo.LastObj++, ObjectDefinition = targetObj, X = targetX, Y = targetY, ScaleX = 1, ScaleY = 1, Color = 0xFFFFFFFF }});
            }}
            break;
        }}
    }}
    if (!rm.GameObjects.Any(g => g.ObjectDefinition == targetObj))
    {{
        rm.GameObjects.Add(new UndertaleRoom.GameObject() {{ InstanceID = Data.GeneralInfo.LastObj++, ObjectDefinition = targetObj, X = targetX, Y = targetY, ScaleX = 1, ScaleY = 1, Color = 0xFFFFFFFF }});
    }}
}}

UndertaleModLib.Compiler.CodeImportGroup importGroup = new(Data) {{ AutoCreateAssets = true, MainThreadAction = MainThreadAction }};

{mods_code}

// Parcheo conjunto de GameOver (Create_0)
var goCreate = Data.Code.ByName("gml_Object_oGameOver_Create_0");
if (goCreate != null)
{{
    try
    {{
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), goCreate, Data.ToolInfo.DecompilerSettings).DecompileToString();
        string target = "Tip = \"You passed out.\";";
        if (decomp.Contains(target)) importGroup.QueueReplace(goCreate, decomp.Replace(target, @"{chained_create}"));
    }}
    catch {{ }}
}}

// Parcheo conjunto de GameOver (Draw_0)
var goDraw = Data.Code.ByName("gml_Object_oGameOver_Draw_0");
if (goDraw != null)
{{
    try
    {{
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), goDraw, Data.ToolInfo.DecompilerSettings).DecompileToString();
        string target = "if (global.Jumpscare == -1)";
        if (decomp.Contains(target)) importGroup.QueueReplace(goDraw, decomp.Replace(target, @"{chained_draw}"));
    }}
    catch {{ }}
}}

importGroup.Import();

// Guardado automático en data.win
try
{{
    string targetWin = Path.Combine(gameDir, "data.win");
    using (FileStream fs = new FileStream(targetWin, FileMode.Create, FileAccess.Write))
    {{
        UndertaleIO.Write(fs, Data);
    }}
    Console.WriteLine("data.win guardado exitosamente en: " + targetWin);
}}
catch (Exception ex)
{{
    Console.WriteLine("Aviso al guardar data.win: " + ex.Message);
}}

{post_injection_msg}

// Clases del Atlas Packer
public class TextureInfo {{ public string Source; public int Width; public int Height; public int TargetX; public int TargetY; public int BoundingWidth; public int BoundingHeight; public MagickImage Image; }}
public enum SplitType {{ Horizontal, Vertical }}
public struct Rect {{ public int X {{ get; set; }} public int Y {{ get; set; }} public int Width {{ get; set; }} public int Height {{ get; set; }} }}
public class Node {{ public Rect Bounds; public TextureInfo Texture; public SplitType SplitType; }}
public class Atlas {{ public int Width; public int Height; public List<Node> Nodes; }}
public class Packer
{{
    public List<TextureInfo> SourceTextures = new List<TextureInfo>();
    public int Padding;
    public int AtlasSize;
    public List<Atlas> Atlasses;
    public void Process(string _SourceDir, string _Pattern, int _AtlasSize, int _Padding, bool _DebugMode)
    {{
        Padding = _Padding; AtlasSize = _AtlasSize;
        DirectoryInfo di = new(_SourceDir);
        foreach (FileInfo fi in di.GetFiles(_Pattern, SearchOption.AllDirectories))
        {{
            MagickImage img = new(fi.FullName);
            SourceTextures.Add(new TextureInfo {{ Source = fi.FullName, Width = (int)img.Width, Height = (int)img.Height, BoundingWidth = (int)img.Width, BoundingHeight = (int)img.Height, TargetX = 0, TargetY = 0, Image = img }});
        }}
        List<TextureInfo> textures = SourceTextures.ToList();
        Atlasses = new List<Atlas>();
        while (textures.Count > 0)
        {{
            Atlas atlas = new Atlas() {{ Width = _AtlasSize, Height = _AtlasSize }};
            List<TextureInfo> leftovers = LayoutAtlas(textures, atlas);
            Atlasses.Add(atlas);
            textures = leftovers;
        }}
    }}
    public void SaveAtlasses(string _Destination)
    {{
        int atlasCount = 0;
        string prefix = Path.Join(Path.GetDirectoryName(_Destination), Path.GetFileNameWithoutExtension(_Destination));
        foreach (Atlas atlas in Atlasses)
        {{
            string atlasName = $"{{prefix}}{{atlasCount:000}}.png";
            using MagickImage img = new(MagickColors.Transparent, (uint)atlas.Width, (uint)atlas.Height);
            foreach (Node n in atlas.Nodes)
            {{
                if (n.Texture != null)
                {{
                    using IMagickImage<byte> res = TextureWorker.ResizeImage(n.Texture.Image, n.Bounds.Width, n.Bounds.Height);
                    img.Composite(res, n.Bounds.X, n.Bounds.Y, CompositeOperator.Copy);
                }}
            }}
            TextureWorker.SaveImageToFile(img, atlasName);
            atlasCount++;
        }}
    }}
    private List<TextureInfo> LayoutAtlas(List<TextureInfo> _Textures, Atlas _Atlas)
    {{
        List<Node> freeList = new List<Node>();
        List<TextureInfo> textures = _Textures.ToList();
        _Atlas.Nodes = new List<Node>();
        freeList.Add(new Node {{ Bounds = new Rect {{ Width = _Atlas.Width, Height = _Atlas.Height }}, SplitType = SplitType.Horizontal }});
        while (freeList.Count > 0 && textures.Count > 0)
        {{
            Node node = freeList[0]; freeList.RemoveAt(0);
            TextureInfo bestFit = textures.FirstOrDefault(ti => ti.Width <= node.Bounds.Width && ti.Height <= node.Bounds.Height);
            if (bestFit != null)
            {{
                Node n1 = new Node {{ Bounds = new Rect {{ X = node.Bounds.X + bestFit.Width + Padding, Y = node.Bounds.Y, Width = node.Bounds.Width - bestFit.Width - Padding, Height = bestFit.Height }}, SplitType = SplitType.Vertical }};
                Node n2 = new Node {{ Bounds = new Rect {{ X = node.Bounds.X, Y = node.Bounds.Y + bestFit.Height + Padding, Width = node.Bounds.Width, Height = node.Bounds.Height - bestFit.Height - Padding }}, SplitType = SplitType.Horizontal }};
                if (n1.Bounds.Width > 0 && n1.Bounds.Height > 0) freeList.Add(n1);
                if (n2.Bounds.Width > 0 && n2.Bounds.Height > 0) freeList.Add(n2);
                node.Texture = bestFit; node.Bounds.Width = bestFit.Width; node.Bounds.Height = bestFit.Height;
                textures.Remove(bestFit);
            }}
            _Atlas.Nodes.Add(node);
        }}
        return textures;
    }}
}}
"#,
            num_mods = mods.len(),
            game_dir = game_dir.display(),
            mods_code = mods_code,
            chained_create = chained_create.replace('"', "\"\""),
            chained_draw = chained_draw.replace('"', "\"\""),
            post_injection_msg = post_injection_msg,
        )
    }

    /// Genera el script .csx COMPLETO (con Atlas Packer anti-pepinillos, Inyección en 6 Salas y Parcheo de oGameOver)
    pub fn generate_csx_installer_code(proj: &AnimatronicProject, version: u32) -> String {
        let id = proj.internal_id.trim();
        let import_dir = proj.workspace_dir.join(format!("{}_Sprites_To_Import", id));
        let gml_dir = proj.workspace_dir.join(format!("{}_GML_Code", id));
        let go_tip = proj.custom_night.gameover_tip.replace('"', "\\\"");

        format!(
            r#"// ==============================================================
// Generado automáticamente por FrickEditor IDE — Instalar_{id}_V{ver}.csx
// Auditado: Incluye Atlas Packer, 6 Salas, oGameOver y Pause Review
// ==============================================================
using System;
using System.IO;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using UndertaleModLib.Util;
using UndertaleModLib.Models;
using ImageMagick;

EnsureDataLoaded();

string importFolder = @"{import_dir}";
string codeDir = @"{gml_dir}";

// 1. EMPAQUETADO DE SPRITES EN ATLAS (ANTI-PEPINILLOS, SIN TRIM CORRUPTO)
if (Directory.Exists(importFolder))
{{
    Regex sprFrameRegex = new(@"^(.+?)(?:_(\d+))$", RegexOptions.Compiled);
    string packDir = Path.Join(ExePath, "Packager");
    Directory.CreateDirectory(packDir);
    string outName = Path.Join(packDir, "atlas_{id}_v{ver}.txt");

    Packer packer = new Packer();
    packer.Process(importFolder, "*.png", 2048, 2, false);
    packer.SaveAtlasses(outName);

    int lastTextPage = Data.EmbeddedTextures.Count - 1;
    int lastTextPageItem = Data.TexturePageItems.Count - 1;
    string prefix = Path.Join(Path.GetDirectoryName(outName), Path.GetFileNameWithoutExtension(outName));
    int atlasCount = 0;

    foreach (Atlas atlas in packer.Atlasses)
    {{
        string atlasName = $"{{prefix}}{{atlasCount:000}}.png";
        using MagickImage atlasImage = TextureWorker.ReadBGRAImageFromFile(atlasName);

        UndertaleEmbeddedTexture texture = new();
        texture.Name = new UndertaleString($"Texture {{++lastTextPage}}");
        texture.TextureData.Image = GMImage.FromMagickImage(atlasImage).ConvertToPng();
        Data.EmbeddedTextures.Add(texture);

        foreach (Node n in atlas.Nodes)
        {{
            if (n.Texture != null)
            {{
                UndertaleTexturePageItem texturePageItem = new();
                texturePageItem.Name = new UndertaleString($"PageItem {{++lastTextPageItem}}");
                texturePageItem.SourceX = (ushort)n.Bounds.X;
                texturePageItem.SourceY = (ushort)n.Bounds.Y;
                texturePageItem.SourceWidth = (ushort)n.Bounds.Width;
                texturePageItem.SourceHeight = (ushort)n.Bounds.Height;
                texturePageItem.TargetX = 0;
                texturePageItem.TargetY = 0;
                texturePageItem.TargetWidth = (ushort)n.Bounds.Width;
                texturePageItem.TargetHeight = (ushort)n.Bounds.Height;
                texturePageItem.BoundingWidth = (ushort)n.Texture.BoundingWidth;
                texturePageItem.BoundingHeight = (ushort)n.Texture.BoundingHeight;
                texturePageItem.TexturePage = texture;
                Data.TexturePageItems.Add(texturePageItem);

                string stripped = Path.GetFileNameWithoutExtension(n.Texture.Source);
                string spriteName = stripped;
                int frame = 0;
                var spriteParts = sprFrameRegex.Match(stripped);
                if (spriteParts.Success)
                {{
                    spriteName = spriteParts.Groups[1].Value;
                    int.TryParse(spriteParts.Groups[2].Value, out frame);
                }}

                UndertaleSprite.TextureEntry texentry = new() {{ Texture = texturePageItem }};
                UndertaleSprite sprite = Data.Sprites.ByName(spriteName);
                if (sprite is null)
                {{
                    UndertaleSprite newSprite = new();
                    newSprite.Name = Data.Strings.MakeString(spriteName);
                    newSprite.Width = (uint)n.Texture.BoundingWidth;
                    newSprite.Height = (uint)n.Texture.BoundingHeight;
                    newSprite.MarginLeft = 0;
                    newSprite.MarginRight = n.Bounds.Width - 1;
                    newSprite.MarginTop = 0;
                    newSprite.MarginBottom = n.Bounds.Height - 1;
                    newSprite.OriginX = 0;
                    newSprite.OriginY = 0;
                    for (int f = 0; f < frame; f++) newSprite.Textures.Add(null);
                    newSprite.Textures.Add(texentry);
                    Data.Sprites.Add(newSprite);
                }}
                else
                {{
                    sprite.Width = (uint)n.Texture.BoundingWidth;
                    sprite.Height = (uint)n.Texture.BoundingHeight;
                    sprite.OriginX = 0;
                    sprite.OriginY = 0;
                    while (frame >= sprite.Textures.Count) sprite.Textures.Add(texentry);
                    sprite.Textures[frame] = texentry;
                }}
            }}
        }}
        atlasCount++;
    }}
}}

// 2. CREACIÓN DE LOS 4 OBJETOS DEL ANIMATRÓNICO
UndertaleGameObject objAnim = Data.GameObjects.ByName("o{id}_BackWindow");
if (objAnim == null) {{ objAnim = new() {{ Name = Data.Strings.MakeString("o{id}_BackWindow"), Visible = true }}; Data.GameObjects.Add(objAnim); }}

UndertaleGameObject objDeath = Data.GameObjects.ByName("o{id}_DeathScreen");
if (objDeath == null) {{ objDeath = new() {{ Name = Data.Strings.MakeString("o{id}_DeathScreen"), Visible = true, Persistent = true }}; Data.GameObjects.Add(objDeath); }}

UndertaleGameObject objCNTile = Data.GameObjects.ByName("o{id}_CustomNightTile");
if (objCNTile == null) {{ objCNTile = new() {{ Name = Data.Strings.MakeString("o{id}_CustomNightTile"), Visible = true }}; Data.GameObjects.Add(objCNTile); }}

UndertaleGameObject objReview = Data.GameObjects.ByName("o{id}_ReviewHandler");
if (objReview == null) {{ objReview = new() {{ Name = Data.Strings.MakeString("o{id}_ReviewHandler"), Visible = true }}; Data.GameObjects.Add(objReview); }}

// 3. COMPILACIÓN DE LOS SCRIPTS GML
UndertaleModLib.Compiler.CodeImportGroup importGroup = new(Data) {{ AutoCreateAssets = true, MainThreadAction = MainThreadAction }};
importGroup.QueueReplace(objAnim.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_BackWindow_Create_0.gml")));
importGroup.QueueReplace(objAnim.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_BackWindow_Step_0.gml")));
importGroup.QueueReplace(objAnim.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_BackWindow_Draw_0.gml")));
importGroup.QueueReplace(objAnim.EventHandlerFor(EventType.Draw, EventSubtypeDraw.DrawGUI, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_BackWindow_Draw_64.gml")));
importGroup.QueueReplace(objAnim.EventHandlerFor(EventType.CleanUp, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_BackWindow_CleanUp_0.gml")));

importGroup.QueueReplace(objDeath.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_DeathScreen_Draw_0.gml")));
importGroup.QueueReplace(objDeath.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_DeathScreen_Step_0.gml")));

importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_CustomNightTile_Create_0.gml")));
importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_CustomNightTile_Step_0.gml")));
importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_CustomNightTile_Draw_0.gml")));

importGroup.QueueReplace(objReview.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_ReviewHandler_Create_0.gml")));
importGroup.QueueReplace(objReview.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_ReviewHandler_Step_0.gml")));
importGroup.QueueReplace(objReview.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_o{id}_ReviewHandler_Draw_0.gml")));

// 4. INYECCIÓN EN TODAS LAS SALAS REQUERIDAS
void AddInstanceToRoom(string roomName, UndertaleGameObject targetObj, int targetX, int targetY)
{{
    UndertaleRoom rm = Data.Rooms.ByName(roomName);
    if (rm == null) return;
    foreach (var l in rm.Layers)
    {{
        if (l.LayerType == UndertaleRoom.LayerType.Instances && l.InstancesData != null)
        {{
            if (!l.InstancesData.Instances.Any(i => i.ObjectDefinition == targetObj))
            {{
                l.InstancesData.Instances.Add(new UndertaleRoom.GameObject() {{ InstanceID = Data.GeneralInfo.LastObj++, ObjectDefinition = targetObj, X = targetX, Y = targetY, ScaleX = 1, ScaleY = 1, Color = 0xFFFFFFFF }});
            }}
            break;
        }}
    }}
    if (!rm.GameObjects.Any(g => g.ObjectDefinition == targetObj))
    {{
        rm.GameObjects.Add(new UndertaleRoom.GameObject() {{ InstanceID = Data.GeneralInfo.LastObj++, ObjectDefinition = targetObj, X = targetX, Y = targetY, ScaleX = 1, ScaleY = 1, Color = 0xFFFFFFFF }});
    }}
}}

AddInstanceToRoom("Office", objAnim, 0, 0);
AddInstanceToRoom("Office", objDeath, 0, 0);
AddInstanceToRoom("Office", objReview, 0, 0);
AddInstanceToRoom("Fredbear_Office", objAnim, 0, 0);
AddInstanceToRoom("Fredbear_Office", objReview, 0, 0);
AddInstanceToRoom("GameOver", objDeath, 0, 0);
AddInstanceToRoom("CustomNightMenu", objCNTile, 0, 0);
AddInstanceToRoom("EndlessSelection", objCNTile, 0, 0);
AddInstanceToRoom("EndlessShopSelection_FinalNight", objCNTile, 0, 0);
{props_code}

// 5. PARCHEO DE PANTALLA GAMEOVER (SILUETA CARMESÍ Y TIME LASTED REAL)
var goCreate = Data.Code.ByName("gml_Object_oGameOver_Create_0");
if (goCreate != null)
{{
    try
    {{
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), goCreate, Data.ToolInfo.DecompilerSettings).DecompileToString();
        if (!decomp.Contains("{id}Death"))
        {{
            string target = "Tip = \"You passed out.\";";
            string rep = "if (variable_global_exists(\"{id}Death\") && global.{id}Death) {{ Tip = \"{go_tip}\"; if (variable_global_exists(\"{id}_SavedTime\")) global.TimeLasted = global.{id}_SavedTime; }} else {{ Tip = \"You passed out.\"; }}";
            if (decomp.Contains(target)) importGroup.QueueReplace(goCreate, decomp.Replace(target, rep));
        }}
    }}
    catch {{ }}
}}

var goDraw = Data.Code.ByName("gml_Object_oGameOver_Draw_0");
if (goDraw != null)
{{
    try
    {{
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), goDraw, Data.ToolInfo.DecompilerSettings).DecompileToString();
        if (!decomp.Contains("{id}Death"))
        {{
            string target = "if (global.Jumpscare == -1)";
            string rep = "if (variable_global_exists(\"{id}Death\") && global.{id}Death) {{ var spr_alt = asset_get_index(\"s{id}_head_Alt\"); if (spr_alt == -1) spr_alt = asset_get_index(\"s{id}_head\"); if (spr_alt != -1) {{ var sc = 3.2; var sw = sprite_get_width(spr_alt) * sc; var sh = sprite_get_height(spr_alt) * sc; draw_sprite_ext(spr_alt, 0, (640 - sw) / 2, ((360 - sh) / 2) - 10, sc, sc, 0, make_color_rgb(175, 20, 20), 0.9); }} }} else if (global.Jumpscare == -1)";
            if (decomp.Contains(target)) importGroup.QueueReplace(goDraw, decomp.Replace(target, rep));
        }}
    }}
    catch {{ }}
}}

importGroup.Import();
ScriptMessage("¡{id} (V{ver}) Auditado e Inyectado con Éxito! Presiona Ctrl+S para guardar data.win.");

// 6. CLASES AUXILIARES DEL EMPAQUETADOR DE TEXTURAS
public class TextureInfo {{ public string Source; public int Width; public int Height; public int TargetX; public int TargetY; public int BoundingWidth; public int BoundingHeight; public MagickImage Image; }}
public enum SplitType {{ Horizontal, Vertical }}
public struct Rect {{ public int X {{ get; set; }} public int Y {{ get; set; }} public int Width {{ get; set; }} public int Height {{ get; set; }} }}
public class Node {{ public Rect Bounds; public TextureInfo Texture; public SplitType SplitType; }}
public class Atlas {{ public int Width; public int Height; public List<Node> Nodes; }}
public class Packer
{{
    public List<TextureInfo> SourceTextures = new List<TextureInfo>();
    public int Padding;
    public int AtlasSize;
    public List<Atlas> Atlasses;
    public void Process(string _SourceDir, string _Pattern, int _AtlasSize, int _Padding, bool _DebugMode)
    {{
        Padding = _Padding; AtlasSize = _AtlasSize;
        DirectoryInfo di = new(_SourceDir);
        foreach (FileInfo fi in di.GetFiles(_Pattern, SearchOption.AllDirectories))
        {{
            MagickImage img = new(fi.FullName);
            SourceTextures.Add(new TextureInfo {{ Source = fi.FullName, Width = (int)img.Width, Height = (int)img.Height, BoundingWidth = (int)img.Width, BoundingHeight = (int)img.Height, TargetX = 0, TargetY = 0, Image = img }});
        }}
        List<TextureInfo> textures = SourceTextures.ToList();
        Atlasses = new List<Atlas>();
        while (textures.Count > 0)
        {{
            Atlas atlas = new Atlas() {{ Width = _AtlasSize, Height = _AtlasSize }};
            List<TextureInfo> leftovers = LayoutAtlas(textures, atlas);
            Atlasses.Add(atlas);
            textures = leftovers;
        }}
    }}
    public void SaveAtlasses(string _Destination)
    {{
        int atlasCount = 0;
        string prefix = Path.Join(Path.GetDirectoryName(_Destination), Path.GetFileNameWithoutExtension(_Destination));
        foreach (Atlas atlas in Atlasses)
        {{
            string atlasName = $"{{prefix}}{{atlasCount:000}}.png";
            using MagickImage img = new(MagickColors.Transparent, (uint)atlas.Width, (uint)atlas.Height);
            foreach (Node n in atlas.Nodes)
            {{
                if (n.Texture != null)
                {{
                    using IMagickImage<byte> res = TextureWorker.ResizeImage(n.Texture.Image, n.Bounds.Width, n.Bounds.Height);
                    img.Composite(res, n.Bounds.X, n.Bounds.Y, CompositeOperator.Copy);
                }}
            }}
            TextureWorker.SaveImageToFile(img, atlasName);
            atlasCount++;
        }}
    }}
    private List<TextureInfo> LayoutAtlas(List<TextureInfo> _Textures, Atlas _Atlas)
    {{
        List<Node> freeList = new List<Node>();
        List<TextureInfo> textures = _Textures.ToList();
        _Atlas.Nodes = new List<Node>();
        freeList.Add(new Node {{ Bounds = new Rect {{ Width = _Atlas.Width, Height = _Atlas.Height }}, SplitType = SplitType.Horizontal }});
        while (freeList.Count > 0 && textures.Count > 0)
        {{
            Node node = freeList[0]; freeList.RemoveAt(0);
            TextureInfo bestFit = textures.FirstOrDefault(ti => ti.Width <= node.Bounds.Width && ti.Height <= node.Bounds.Height);
            if (bestFit != null)
            {{
                Node n1 = new Node {{ Bounds = new Rect {{ X = node.Bounds.X + bestFit.Width + Padding, Y = node.Bounds.Y, Width = node.Bounds.Width - bestFit.Width - Padding, Height = bestFit.Height }}, SplitType = SplitType.Vertical }};
                Node n2 = new Node {{ Bounds = new Rect {{ X = node.Bounds.X, Y = node.Bounds.Y + bestFit.Height + Padding, Width = node.Bounds.Width, Height = node.Bounds.Height - bestFit.Height - Padding }}, SplitType = SplitType.Horizontal }};
                if (n1.Bounds.Width > 0 && n1.Bounds.Height > 0) freeList.Add(n1);
                if (n2.Bounds.Width > 0 && n2.Bounds.Height > 0) freeList.Add(n2);
                node.Texture = bestFit; node.Bounds.Width = bestFit.Width; node.Bounds.Height = bestFit.Height;
                textures.Remove(bestFit);
            }}
            _Atlas.Nodes.Add(node);
        }}
        return textures;
    }}
}}
"#,
            id = id,
            ver = version,
            import_dir = import_dir.display(),
            gml_dir = gml_dir.display(),
            go_tip = go_tip,
            props_code = {
                let mut code = String::new();
                if !proj.advanced.maze_entities.is_empty() {
                    let target_room = &proj.advanced.target_maze_room;
                    code.push_str(&format!("\n// Inyección de Props 3D / Laberinto en {target_room} para {id}\n"));
                    for (idx, ent) in proj.advanced.maze_entities.iter().enumerate() {
                        let obj_name = ent.prop_type.gml_object_name();
                        code.push_str(&format!(
                            "UndertaleGameObject prop_{id}_{idx} = Data.GameObjects.ByName(\"{obj_name}\");\n\
                             if (prop_{id}_{idx} != null) AddInstanceToRoom(\"{target_room}\", prop_{id}_{idx}, {x}, {y});\n",
                            id = id,
                            idx = idx,
                            obj_name = obj_name,
                            target_room = target_room,
                            x = ent.x,
                            y = ent.y
                        ));
                    }
                }
                code
            }
        )
    }

    pub fn build_phase1_visuals(proj: &AnimatronicProject) -> Result<String, String> {
        let id = proj.internal_id.trim();
        if id.is_empty() {
            return Err("El ID interno del animatrónico no puede estar vacío.".to_string());
        }

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

        let extras_json = serde_json::to_string_pretty(&proj.gallery_info).map_err(|e| e.to_string())?;
        fs::write(addon_dir.join("extras_info.txt"), extras_json).map_err(|e| e.to_string())?;

        if let Some(portrait_src) = &proj.portrait_500_path {
            let img = image::open(portrait_src).map_err(|e| format!("Error en Portrait: {}", e))?;
            img.resize_exact(500, 500, imageops::FilterType::Lanczos3)
                .save(addon_dir.join("portrait.png"))
                .map_err(|e| e.to_string())?;
        }

        let import_dir = proj.workspace_dir.join(format!("{}_Sprites_To_Import", id));
        fs::create_dir_all(&import_dir).map_err(|e| e.to_string())?;

        if let Some(icon_src) = &proj.icon_48_path {
            let img = image::open(icon_src).map_err(|e| format!("Error en Icono: {}", e))?;
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
            } else if let Some(gray_override) = &proj.icon_gray_override_path {
                if gray_override.exists() {
                    let g_img = image::open(gray_override).map_err(|e| e.to_string())?;
                    g_img
                        .resize_exact(48, 48, imageops::FilterType::Nearest)
                        .save(import_dir.join(format!("sTile_{}_1.png", id)))
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        Self::copy_sprite(&proj.body_sprite, &import_dir, &format!("s{}_body_0.png", id))?;
        Self::copy_sprite(&proj.head_normal_sprite, &import_dir, &format!("s{}_head_0.png", id))?;
        Self::copy_sprite(&proj.head_scare_sprite, &import_dir, &format!("s{}_head_Scare_0.png", id))?;
        Self::copy_sprite(&proj.head_gameover_alt, &import_dir, &format!("s{}_head_Alt_0.png", id))?;
        Self::copy_sprite(&proj.accessory_sprite, &import_dir, &format!("s{}_Glasses_0.png", id))?;
        Self::copy_sprite(&proj.flash_defeat_sprite, &import_dir, &format!("s{}_Flash_0.png", id))?;

        for extra in &proj.extra_sprites {
            let clean_suffix = extra.sprite_suffix.trim();
            if !clean_suffix.is_empty() {
                Self::copy_sprite(&Some(extra.path.clone()), &import_dir, &format!("s{}_{}_0.png", id, clean_suffix))?;
            }
        }

        if let (Some(j0), Some(j1), Some(j2)) = (
            &proj.jumpscare_frame_0,
            &proj.jumpscare_frame_1,
            &proj.jumpscare_frame_2,
        ) {
            Self::precomposite_jumpscare(j0, j1, j2, &import_dir, id)?;
        }

        let id_low = id.to_lowercase();
        Self::copy_audio(&proj.sound_knock_alert, &proj.game_dir, &format!("{}_knock.ogg", id_low))?;
        Self::copy_audio(&proj.sound_voice_line, &proj.game_dir, &format!("{}_amigable.ogg", id_low))?;
        Self::copy_audio(&proj.sound_defeat, &proj.game_dir, &format!("{}_knock2.ogg", id_low))?;
        Self::copy_audio(&proj.sound_jumpscare, &proj.game_dir, &format!("{}_jumpscare.ogg", id_low))?;

        Ok(format!("Fase 1 exportada en addons/{0} y {0}_Sprites_To_Import.", id))
    }

    pub fn clean_from_game(proj: &AnimatronicProject) -> Result<String, String> {
        let id = proj.internal_id.trim();
        if id.is_empty() {
            return Err("ID vacío.".to_string());
        }
        let addon_dir = proj.game_dir.join("addons").join(id);
        if addon_dir.exists() {
            fs::remove_dir_all(&addon_dir).map_err(|e| e.to_string())?;
        }
        let id_low = id.to_lowercase();
        for aud_suffix in ["knock.ogg", "amigable.ogg", "knock2.ogg", "jumpscare.ogg"] {
            let aud_file = proj.game_dir.join(format!("{}_{}", id_low, aud_suffix));
            if aud_file.exists() {
                let _ = fs::remove_file(aud_file);
            }
        }
        Ok(format!("'{}' limpiado del juego (archivos fuente intactos).", id))
    }

    pub fn export_fbmod_package(proj: &AnimatronicProject, dst_zip: &Path) -> Result<String, String> {
        let file = File::create(dst_zip).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let manifest_json = serde_json::to_string_pretty(proj).map_err(|e| e.to_string())?;
        zip.start_file("project_manifest.json", options).map_err(|e| e.to_string())?;
        zip.write_all(manifest_json.as_bytes()).map_err(|e| e.to_string())?;

        for (fname, code) in Self::get_effective_gml_scripts(proj) {
            zip.start_file(format!("gml/{}", fname), options).map_err(|e| e.to_string())?;
            zip.write_all(code.as_bytes()).map_err(|e| e.to_string())?;
        }

        let files_to_pack = [
            ("assets/portrait.png", &proj.portrait_500_path),
            ("assets/icon.png", &proj.icon_48_path),
            ("assets/body.png", &proj.body_sprite),
            ("assets/head_normal.png", &proj.head_normal_sprite),
            ("assets/head_scare.png", &proj.head_scare_sprite),
            ("assets/head_gameover_alt.png", &proj.head_gameover_alt),
            ("assets/accessory.png", &proj.accessory_sprite),
            ("assets/flash_defeat.png", &proj.flash_defeat_sprite),
            ("assets/jumpscare_0.png", &proj.jumpscare_frame_0),
            ("assets/jumpscare_1.png", &proj.jumpscare_frame_1),
            ("assets/jumpscare_2.png", &proj.jumpscare_frame_2),
            ("audio/knock.ogg", &proj.sound_knock_alert),
            ("audio/voice.ogg", &proj.sound_voice_line),
            ("audio/defeat.ogg", &proj.sound_defeat),
            ("audio/jumpscare.ogg", &proj.sound_jumpscare),
        ];

        for (zip_path, opt_src) in files_to_pack {
            if let Some(src) = opt_src {
                if src.exists() {
                    if let Ok(mut f) = File::open(src) {
                        let mut buf = Vec::new();
                        if f.read_to_end(&mut buf).is_ok() {
                            zip.start_file(zip_path, options).map_err(|e| e.to_string())?;
                            zip.write_all(&buf).map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }

        zip.finish().map_err(|e| e.to_string())?;
        Ok(format!("Paquete descargable (.fbmod) exportado en: {}", dst_zip.display()))
    }

    /// Importa un archivo .fbmod descargable, extrayendo sus assets y cargando el proyecto en el IDE
    pub fn import_fbmod_package(zip_path: &Path, workspace_dir: &Path) -> Result<AnimatronicProject, String> {
        let file = File::open(zip_path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

        let mut manifest_str = String::new();
        {
            let mut mf = archive
                .by_name("project_manifest.json")
                .map_err(|_| "El archivo .fbmod no contiene project_manifest.json".to_string())?;
            mf.read_to_string(&mut manifest_str).map_err(|e| e.to_string())?;
        }

        let mut proj: AnimatronicProject = serde_json::from_str(&manifest_str).map_err(|e| e.to_string())?;
        let extract_dir = workspace_dir.join(format!("{}_Imported_Assets", proj.internal_id.trim()));
        fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

        let extract_one = |archive: &mut ZipArchive<File>, zip_entry: &str, out_name: &str| -> Option<std::path::PathBuf> {
            if let Ok(mut entry) = archive.by_name(zip_entry) {
                let out_path = extract_dir.join(out_name);
                if let Ok(mut out_file) = File::create(&out_path) {
                    let _ = std::io::copy(&mut entry, &mut out_file);
                    return Some(out_path);
                }
            }
            None
        };

        proj.portrait_500_path = extract_one(&mut archive, "assets/portrait.png", "portrait.png");
        proj.icon_48_path = extract_one(&mut archive, "assets/icon.png", "icon.png");
        proj.body_sprite = extract_one(&mut archive, "assets/body.png", "body.png");
        proj.head_normal_sprite = extract_one(&mut archive, "assets/head_normal.png", "head_normal.png");
        proj.head_scare_sprite = extract_one(&mut archive, "assets/head_scare.png", "head_scare.png");
        proj.head_gameover_alt = extract_one(&mut archive, "assets/head_gameover_alt.png", "head_gameover_alt.png");
        proj.accessory_sprite = extract_one(&mut archive, "assets/accessory.png", "accessory.png");
        proj.flash_defeat_sprite = extract_one(&mut archive, "assets/flash_defeat.png", "flash_defeat.png");
        proj.jumpscare_frame_0 = extract_one(&mut archive, "assets/jumpscare_0.png", "jumpscare_0.png");
        proj.jumpscare_frame_1 = extract_one(&mut archive, "assets/jumpscare_1.png", "jumpscare_1.png");
        proj.jumpscare_frame_2 = extract_one(&mut archive, "assets/jumpscare_2.png", "jumpscare_2.png");
        proj.sound_knock_alert = extract_one(&mut archive, "audio/knock.ogg", "knock.ogg");
        proj.sound_voice_line = extract_one(&mut archive, "audio/voice.ogg", "voice.ogg");
        proj.sound_defeat = extract_one(&mut archive, "audio/defeat.ogg", "defeat.ogg");
        proj.sound_jumpscare = extract_one(&mut archive, "audio/jumpscare.ogg", "jumpscare.ogg");

        Ok(proj)
    }

    fn copy_sprite(src: &Option<std::path::PathBuf>, dst_dir: &Path, dst_name: &str) -> Result<(), String> {
        if let Some(path) = src {
            if path.exists() {
                fs::copy(path, dst_dir.join(dst_name)).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn copy_audio(src: &Option<std::path::PathBuf>, game_dir: &Path, dst_name: &str) -> Result<(), String> {
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
            imageops::overlay(&mut canvas, head, 0, dy_head);

            let out_name = format!("s{}_Jumpscare_{}.png", id, idx);
            canvas.save(import_dir.join(out_name)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
