use crate::models::AnimatronicProject;
use image::{imageops, DynamicImage, ImageBuffer, Rgba};
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// Plantillas GML traducidas 1:1 del golden_master_v26 (referencia probada en
// producción). El personaje se parametriza AL GENERAR: el placeholder {ID} se
// sustituye por el identificador del proyecto (p.ej. "Murray" -> "Test") y {id}
// por el mismo en minúsculas. {gx}/{gy} = offset de las gafas, {tooltip} = texto
// editable del proyecto. No editar la lógica aquí: cualquier cambio debe
// replicarse en templates/golden_master_v26/gml/.
// ============================================================================

/// oMurray_CustomNightTile -> Step_0: clics ◄ [00] ►, contador central, clic
/// derecho, rueda del ratón y botones globales SET ALL / ADD ALL aislados (golden V21).
const CN_STEP_TEMPLATE: &str = r#"// ==============================================================
// {ID} - Custom Night Tile Logic (Step, V26-aligned)
// Cuadrícula nativa 8x4, flechas (0..16, 16..32, 32..48) y botones globales aislados
// Generado por FrickEditor Studio (golden_master_v26)
// ==============================================================
depth = -1000;
if (!variable_global_exists("{ID}_AI")) global.{ID}_AI = 0;
if (!variable_global_exists("CustomNight_Page")) global.CustomNight_Page = 0;

var snd_click = asset_get_index("sfxClick");
if (snd_click == -1) snd_click = asset_get_index("snd_Button");
var snd_btn = asset_get_index("snd_Button");

var cur_tile_x = tile_x;
var cur_tile_y = tile_y;

// 1. Clics izquierdos principales
if (mouse_check_button_pressed(mb_left)) {
    // A. Botón de cambio de página (344, 326, 110, 26)
    if (mouse_x >= btn_page_x && mouse_x <= btn_page_x + btn_page_w && mouse_y >= btn_page_y && mouse_y <= btn_page_y + btn_page_h) {
        global.CustomNight_Page = 1 - global.CustomNight_Page;
        if (snd_click != -1) audio_play_sound(snd_click, 10, false);

        // Aislar cuadrícula: en Página 2 desactivar tiles vanilla para que no se dibujen ni respondan a botones
        if (global.CustomNight_Page == 1) {
            instance_deactivate_object(oCustomNightTile);
        } else {
            instance_activate_object(oCustomNightTile);
        }
    }
    // B. Controles de {ID} en Página 2 (Slot 0,0 en 32, 12)
    // ◄ en 32..47 (0..15), Contador en 48..63 (16..31), ► en 64..79 (32..47)
    else if (global.CustomNight_Page == 1) {
        if (mouse_y >= cur_tile_y + 48 && mouse_y <= cur_tile_y + 64) {
            // Flecha Izquierda ◄ (Decrementar)
            if (mouse_x >= cur_tile_x && mouse_x < cur_tile_x + 16) {
                if (global.{ID}_AI > 0) {
                    global.{ID}_AI--;
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                }
            }
            // Flecha Derecha ► (Incrementar)
            else if (mouse_x >= cur_tile_x + 32 && mouse_x <= cur_tile_x + 48) {
                if (global.{ID}_AI < 20) {
                    global.{ID}_AI++;
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                }
            }
            // Clic en el número central (Alternar 0 / 20)
            else if (mouse_x >= cur_tile_x + 16 && mouse_x < cur_tile_x + 32) {
                global.{ID}_AI = (global.{ID}_AI > 0) ? 0 : 20;
                if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
            }
        }

        // C. Botones globales del panel derecho (SET ALL 0, ADD ALL 1, ADD ALL 5, SET ALL 20)
        // EXCLUSIVO de la Página 2: Solo afecta a {ID} y no toca a los vanilla
        if (mouse_x >= 478 && mouse_x <= 612) {
            // Fila 1: SET ALL 0 y ADD ALL 1 (Y: 35..71)
            if (mouse_y >= 35 && mouse_y <= 71) {
                if (mouse_x < 545) {
                    global.{ID}_AI = 0; // SET ALL 0
                    if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
                } else {
                    global.{ID}_AI = min(20, global.{ID}_AI + 1); // ADD ALL 1
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                }
            }
            // Fila 2: ADD ALL 5 y SET ALL 20 (Y: 72..108)
            else if (mouse_y >= 72 && mouse_y <= 108) {
                if (mouse_x < 545) {
                    global.{ID}_AI = min(20, global.{ID}_AI + 5); // ADD ALL 5
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                } else {
                    global.{ID}_AI = 20; // SET ALL 20
                    if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
                }
            }
        }

        // D. Al presionar START NIGHT en el panel derecho (X >= 480, Y >= 300), reactivar tiles vanilla
        if (mouse_x >= 480 && mouse_y >= 300) {
            instance_activate_object(oCustomNightTile);
        }
    }
}

// 2. Clic derecho en el Tile de {ID} para alternar 0 / 20
if (global.CustomNight_Page == 1 && mouse_check_button_pressed(mb_right)) {
    if (mouse_x >= cur_tile_x && mouse_x <= cur_tile_x + 48 && mouse_y >= cur_tile_y && mouse_y <= cur_tile_y + 64) {
        global.{ID}_AI = (global.{ID}_AI > 0) ? 0 : 20;
        if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
    }
}

// 3. Rueda del ratón encima del Tile de {ID}
if (global.CustomNight_Page == 1) {
    if (mouse_x >= cur_tile_x && mouse_x <= cur_tile_x + 48 && mouse_y >= cur_tile_y && mouse_y <= cur_tile_y + 64) {
        if (mouse_wheel_up()) {
            if (global.{ID}_AI < 20) {
                global.{ID}_AI++;
                if (snd_click != -1) audio_play_sound(snd_click, 0, false);
            }
        } else if (mouse_wheel_down()) {
            if (global.{ID}_AI > 0) {
                global.{ID}_AI--;
                if (snd_click != -1) audio_play_sound(snd_click, 0, false);
            }
        }
    }
}
"#;

/// oMurray_CustomNightTile -> Draw_0: cuadrícula nativa 8x4, slot (0,0) destacado
/// con flechas ◄ ► y contador, tooltip Windows 95 y botón PAGE 1/2 (golden V22).
const CN_DRAW_TEMPLATE: &str = r#"// ==============================================================
// {ID} - Custom Night Tile Render (Draw, V26-aligned)
// Cuadrícula estilizada (8x4 slots), ◄ [00] ► nativo y botón PAGE 1/2 alineado
// Generado por FrickEditor Studio (golden_master_v26)
// ==============================================================
var fnt = asset_get_index("fnt_Small");
if (fnt == -1) fnt = asset_get_index("fnt_Dialogue");
if (fnt != -1) draw_set_font(fnt);

// 1. Si estamos en la Página 2 ({ID}), dibujar la cuadrícula completa de 8x4
if (global.CustomNight_Page == 1) {
    // Fondo oscuro idéntico a la cuadrícula original
    draw_set_color(c_black);
    draw_rectangle(28, 0, 464, 360, false);

    var spr_base = asset_get_index("sTileBase");
    var spr_arrows = asset_get_index("sCustomNightArrows");
    var spr_counter = asset_get_index("sCustomNightCounter");

    // Dibujar la cuadrícula oficial de 4 filas x 8 columnas (32 slots idénticos a vanilla)
    for (var row = 0; row < 4; row++) {
        for (var col = 0; col < 8; col++) {
            var sx = 32 + col * 54;
            var sy = 12 + row * 72;

            if (row == 0 && col == 0) {
                // SLOT (0,0): {ID}
                if (spr_base != -1) {
                    draw_sprite(spr_base, 0, sx, sy);
                } else {
                    draw_set_color(make_color_rgb(192, 192, 192));
                    draw_rectangle(sx, sy, sx + 47, sy + 47, false);
                }

                // Retrato de {ID}
                var spr_murray = asset_get_index("sTile_{ID}");
                if (spr_murray != -1) {
                    var is_dc = (variable_global_exists("{ID}_DeathCoined") && global.{ID}_DeathCoined);
                    if (global.{ID}_AI > 0 && !is_dc) {
                        draw_sprite(spr_murray, 0, sx, sy);
                    } else if (sprite_get_number(spr_murray) > 1) {
                        draw_sprite(spr_murray, 1, sx, sy);
                    } else {
                        draw_sprite_ext(spr_murray, 0, sx, sy, 1, 1, 0, c_gray, 1.0);
                    }
                }

                // Formato idéntico al juego original:
                // ◄ a la izquierda (0..16), Contador en el centro (16..32), ► a la derecha (32..48)
                if (spr_arrows != -1) {
                    // Flecha izquierda ◄
                    draw_sprite_part(spr_arrows, 0, 0, 0, 16, 16, sx, sy + 48);
                    // Flecha derecha ►
                    draw_sprite_part(spr_arrows, 0, 16, 0, 16, 16, sx + 32, sy + 48);
                }
                if (spr_counter != -1) {
                    // Contador digital centrado exactamente en +16 px
                    draw_sprite(spr_counter, global.{ID}_AI, sx + 16, sy + 48);
                }
            } else {
                // SLOTS VACÍOS DE LA CUADRÍCULA ESTILIZADOS
                // Marco exterior de 48x48 con bisel sutil estilo Windows 95
                draw_set_color(make_color_rgb(18, 18, 18));
                draw_rectangle(sx, sy, sx + 47, sy + 47, false);

                // Bisel sutil (luz arriba/izq, sombra abajo/der)
                draw_set_color(make_color_rgb(50, 50, 50));
                draw_line(sx, sy, sx + 47, sy);
                draw_line(sx, sy, sx, sy + 47);
                draw_set_color(make_color_rgb(10, 10, 10));
                draw_line(sx, sy + 47, sx + 47, sy + 47);
                draw_line(sx + 47, sy, sx + 47, sy + 47);

                // Icono "+" minimalista y elegante en el centro del slot
                draw_set_color(make_color_rgb(38, 38, 38));
                draw_line(sx + 21, sy + 24, sx + 27, sy + 24);
                draw_line(sx + 24, sy + 21, sx + 24, sy + 27);

                // Marco inferior para flechas/contador (48x16)
                draw_set_color(make_color_rgb(14, 14, 14));
                draw_rectangle(sx, sy + 48, sx + 47, sy + 63, false);
                draw_set_color(make_color_rgb(35, 35, 35));
                draw_line(sx, sy + 48, sx + 47, sy + 48);
                draw_line(sx, sy + 48, sx, sy + 63);
                draw_set_color(make_color_rgb(8, 8, 8));
                draw_line(sx, sy + 63, sx + 47, sy + 63);
                draw_line(sx + 47, sy + 48, sx + 47, sy + 63);

                // Guión sutil en el contador vacío
                draw_set_color(make_color_rgb(32, 32, 32));
                draw_line(sx + 21, sy + 56, sx + 27, sy + 56);
            }
        }
    }

    // Tooltip oficial Windows 95 al pasar el cursor sobre {ID}
    var show_desc = true;
    if (variable_global_exists("ShowDescriptions")) show_desc = global.ShowDescriptions;

    if (show_desc && mouse_x >= 32 && mouse_x <= 80 && mouse_y >= 12 && mouse_y <= 76) {
        var tip_txt = "{tooltip}";
        var tw = 240;
        var th = 88;
        var tx = clamp(mouse_x + 12, 32, 450 - tw);
        var ty = clamp(mouse_y + 12, 12, 350 - th);

        // Caja gris biselada nativa
        draw_set_color(make_color_rgb(198, 198, 198));
        draw_rectangle(tx, ty, tx + tw, ty + th, false);
        draw_set_color(c_white);
        draw_line(tx, ty, tx + tw, ty);
        draw_line(tx, ty, tx, ty + th);
        draw_set_color(make_color_rgb(80, 80, 80));
        draw_line(tx, ty + th, tx + tw, ty + th);
        draw_line(tx + tw, ty, tx + tw, ty + th);
        draw_set_color(c_black);
        draw_rectangle(tx - 1, ty - 1, tx + tw + 1, ty + th + 1, true);

        draw_set_halign(fa_left);
        draw_set_valign(fa_top);
        draw_set_color(c_black);
        draw_text_ext(tx + 7, ty + 7, tip_txt, 14, tw - 14);
    }
}

// 2. Botón cuadrado de cambio de página (344, 326, 110, 26) en la misma posición en ambas páginas
var hov_btn = (mouse_x >= btn_page_x && mouse_x <= btn_page_x + btn_page_w && mouse_y >= btn_page_y && mouse_y <= btn_page_y + btn_page_h);
var pressed = (hov_btn && mouse_check_button(mb_left));

draw_set_color(hov_btn ? make_color_rgb(215, 215, 215) : make_color_rgb(192, 192, 192));
draw_rectangle(btn_page_x, btn_page_y, btn_page_x + btn_page_w, btn_page_y + btn_page_h, false);

// Biselado oficial
draw_set_color(pressed ? make_color_rgb(80, 80, 80) : c_white);
draw_line(btn_page_x, btn_page_y, btn_page_x + btn_page_w, btn_page_y);
draw_line(btn_page_x, btn_page_y, btn_page_x, btn_page_y + btn_page_h);

draw_set_color(pressed ? c_white : make_color_rgb(80, 80, 80));
draw_line(btn_page_x, btn_page_y + btn_page_h, btn_page_x + btn_page_w, btn_page_y + btn_page_h);
draw_line(btn_page_x + btn_page_w, btn_page_y, btn_page_x + btn_page_w, btn_page_y + btn_page_h);

draw_set_color(c_black);
draw_rectangle(btn_page_x - 1, btn_page_y - 1, btn_page_x + btn_page_w + 1, btn_page_y + btn_page_h + 1, true);

// Icono triangular y texto centrado
draw_set_valign(fa_middle);
draw_set_color(c_black);

if (global.CustomNight_Page == 0) {
    // En Página 1: Botón para ir a PAGE 2
    draw_set_halign(fa_center);
    draw_text(btn_page_x + 48, btn_page_y + 13, "PAGE 2");

    var tri_x = btn_page_x + 88;
    var tri_y = btn_page_y + 13;
    draw_triangle(tri_x - 4, tri_y - 5, tri_x - 4, tri_y + 5, tri_x + 4, tri_y, false);
} else {
    // En Página 2: Botón para volver a PAGE 1
    draw_set_halign(fa_center);
    draw_text(btn_page_x + 62, btn_page_y + 13, "PAGE 1");

    var tri_x = btn_page_x + 22;
    var tri_y = btn_page_y + 13;
    draw_triangle(tri_x + 4, tri_y - 5, tri_x + 4, tri_y + 5, tri_x - 4, tri_y, false);
}

draw_set_halign(fa_left);
draw_set_valign(fa_top);
draw_set_color(c_white);
"#;

/// oMurray_BackWindow -> Step_0: máquina de estados completa (0..4) — ventana derecha
/// (1462, 1085), mecánica de persiana con Sun/Moon, gafas, Death Coin y jumpscare (golden V25).
const BW_STEP_TEMPLATE: &str = r#"// ==============================================================
// {ID} - Back Window Logic (Step, V26-aligned)
// Ventana derecha (1462, 1085), animación de cuerpo (frames 0..5),
// audio único de golpe snd_{id}_knock.ogg, audio alegre de jumpscare snd_{id}_jumpscare.ogg,
// y mecánica elegante de persiana: Si el usuario la levanta se queda arriba; al ahuyentarlo cae sola.
// Generado por FrickEditor Studio (golden_master_v26)
// ==============================================================
var cx = 1462;
var cy = 1085;
var my_blind = instance_nearest(cx, cy, oWindowBlinds);

// 1. Contador de supervivencia a 60 FPS para fijar TimeOfDeath exacto
survival_frames++;
var cur_sec = survival_frames / 60.0;
if (variable_global_exists("TimerMinutes") && variable_global_exists("TimerSeconds")) {
    var _m = global.TimerMinutes;
    var _s = global.TimerSeconds;
    var _dec = (variable_global_exists("TimerDecimal") ? global.TimerDecimal : 0);
    cur_sec = (_m * 60) + _s + (_dec / 100.0);
}
global.{ID}_SavedTimeSeconds = cur_sec;

// 2. Verificación de Death Coin
var deathcoined = false;
if (variable_global_exists("{ID}_DeathCoined") && global.{ID}_DeathCoined) deathcoined = true;
if (variable_global_exists("{ID}_Disabled") && global.{ID}_Disabled) deathcoined = true;
if (variable_global_exists("DeathCoin_Target") && global.DeathCoin_Target == "{ID}") deathcoined = true;
if (state == 1 && mouse_check_button_pressed(mb_left)) {
    if (variable_global_exists("HeldItem") && (global.HeldItem == "DeathCoin" || global.HeldItem == "Death Coin" || global.HeldItem == "oShopItem_DeathCoin")) {
        deathcoined = true;
    }
}

if (deathcoined) {
    if ({ID}Body != noone && instance_exists({ID}Body)) { instance_destroy({ID}Body); {ID}Body = noone; }
    if ({ID}Head != noone && instance_exists({ID}Head)) { instance_destroy({ID}Head); {ID}Head = noone; }
    if ({ID}Glasses != noone && instance_exists({ID}Glasses)) { instance_destroy({ID}Glasses); {ID}Glasses = noone; }
    glasses_flying = false;
    if (instance_exists(my_blind)) {
        my_blind.ForcedOpen = false;
        my_blind.Open = false;
    }
    var snd_dc = asset_get_index("sfxDeathCoin");
    if (snd_dc == -1) snd_dc = asset_get_index("sfxIgnitedFreddy_Hit1");
    if (snd_dc != -1) audio_play_sound(snd_dc, 10, false);
    global.{ID}_DeathCoined = true;
    global.{ID}_Disabled = true;
    instance_destroy();
    exit;
}

// 3. Física de las gafas al salir disparadas
if (glasses_flying && {ID}Glasses != noone && instance_exists({ID}Glasses)) {
    {ID}Glasses.x += glasses_vx;
    {ID}Glasses.y += glasses_vy;
    glasses_vy += 1.2;
    if ({ID}Glasses.y > 1300) {
        instance_destroy({ID}Glasses);
        {ID}Glasses = noone;
        glasses_flying = false;
    }
}

// 4. ESTADO 0: Fuera de la ventana trasera derecha
if (state == 0) {
    if ({ID}Body != noone && instance_exists({ID}Body)) { instance_destroy({ID}Body); {ID}Body = noone; }
    if ({ID}Head != noone && instance_exists({ID}Head)) { instance_destroy({ID}Head); {ID}Head = noone; }
    if ({ID}Glasses != noone && instance_exists({ID}Glasses)) { instance_destroy({ID}Glasses); {ID}Glasses = noone; }
    glasses_flying = false;

    knock_timer--;
    if (knock_timer <= 0) {
        // {ID} llega a la ventana derecha (1462, 1085)
        state = 1;
        var ai = 20;
        if (variable_global_exists("{ID}_AI")) ai = clamp(global.{ID}_AI, 0, 20);
        attack_timer = round(lerp(600, 320, ai / 20.0));
        has_glasses = true;
        remove_cooldown = 0;
        head_recoil_x = 0;
        head_recoil_y = 0;
        body_frame = 0;

        var spr_body = asset_get_index("s{ID}_body");
        var spr_head = asset_get_index("s{ID}_head");
        var spr_glasses = asset_get_index("s{ID}_Glasses");

        // El cuerpo animado centrado exactamente en la ventana
        {ID}Body = instance_create_layer(cx - 200, cy - 23, "Behind_Wall", oOfficeSprite);
        if (spr_body != -1) {
            {ID}Body.sprite_index = spr_body;
            {ID}Body.image_index = 0;
            {ID}Body.image_speed = 0;
        }
        {ID}Body.DrawDepth = 48;

        {ID}Head = instance_create_layer(cx - 77, cy, "Behind_Wall", oOfficeSprite);
        if (spr_head != -1) {ID}Head.sprite_index = spr_head;
        {ID}Head.DrawDepth = 50;

        {ID}Glasses = instance_create_layer(cx - 77 + {gx}, cy + {gy}, "Behind_Wall", oOfficeSprite);
        if (spr_glasses != -1) {ID}Glasses.sprite_index = spr_glasses;
        {ID}Glasses.DrawDepth = 52;

        // Al llegar, la persiana NO se fuerza abierta automáticamente:
        // si está cerrada, se mantiene cerrada para respetar la seguridad con Sun & Moon
        if (instance_exists(my_blind)) {
            my_blind.ForcedOpen = false;
        }

        // ÚNICO SONIDO PERMITIDO AL APARECER: snd_{id}_knock.ogg
        var _p_knock = "snd_{id}_knock.ogg";
        if (!file_exists(_p_knock)) _p_knock = working_directory + "snd_{id}_knock.ogg";
        if (!file_exists(_p_knock)) _p_knock = program_directory + "snd_{id}_knock.ogg";
        if (file_exists(_p_knock)) {
            var str_k = audio_create_stream(_p_knock);
            audio_play_sound(str_k, 10, false);
        } else {
            var snd_k = asset_get_index("snd_{id}_knock");
            if (snd_k == -1) snd_k = asset_get_index("sfxKnock");
            if (snd_k != -1) audio_play_sound(snd_k, 10, false);
        }
    }
}

// 5. ESTADO 1: {ID} acechando en la ventana derecha
if (state == 1) {
    // MECÁNICA ELEGANTE DE PERSIANA:
    // Si la persiana está cerrada: {ID} NO la fuerza abierta y no puede atacar (timer pausado).
    // Si el usuario la levanta: {ID} la sostiene alzada firmemente (ForcedOpen = true).
    if (instance_exists(my_blind)) {
        if (!my_blind.Open) {
            attack_timer = max(attack_timer, 180);
            my_blind.ForcedOpen = false;
        } else {
            // El usuario levantó la persiana: {ID} la sostiene alzada cómodamente
            my_blind.ForcedOpen = true;
            my_blind.Open = true;
            attack_timer--;
        }
    } else {
        attack_timer--;
    }

    if (remove_cooldown > 0) {
        remove_cooldown--;
    }

    head_recoil_x = lerp(head_recoil_x, 0, 0.15);
    head_recoil_y = lerp(head_recoil_y, 0, 0.15);

    var torso_bob = sin(current_time * 0.003) * 2.0;

    // Reproducir la animación de levantar la persiana (frames 0..5) y sostenerla en frame 5
    if ({ID}Body != noone && instance_exists({ID}Body)) {
        if (body_frame < 5) {
            body_frame += 0.25;
            if (body_frame > 5) body_frame = 5;
        }
        {ID}Body.image_index = floor(body_frame);
        {ID}Body.image_speed = 0;
        {ID}Body.x = cx - 200;
        {ID}Body.y = (cy - 23) + torso_bob;
    }

    var head_bob = sin((current_time * 0.003) + 0.5) * 2.5;
    if ({ID}Head != noone && instance_exists({ID}Head)) {
        {ID}Head.x = (cx - 77) + head_recoil_x;
        {ID}Head.y = cy + head_bob + head_recoil_y;
    }

    if (has_glasses && !glasses_flying && {ID}Glasses != noone && instance_exists({ID}Glasses)) {
        {ID}Glasses.x = (cx - 77 + {gx}) + head_recoil_x;
        {ID}Glasses.y = (cy + {gy}) + head_bob + head_recoil_y;
    }

    // Proyección de cursor en perspectiva 3D (o3DMouse)
    var mx = mouse_x;
    var my = mouse_y;
    if (instance_exists(asset_get_index("o3DMouse"))) {
        var _mobj = instance_find(asset_get_index("o3DMouse"), 0);
        if (_mobj != noone) {
            mx = _mobj.x;
            my = _mobj.y;
        }
    }

    // Hitboxes en la ventana derecha (1462, 1085)
    var hover_glasses = false;
    if (mx >= (cx - 55) && mx <= (cx + 55) && my >= (cy + 15) && my <= (cy + 85)) hover_glasses = true;
    if ({ID}Glasses != noone && instance_exists({ID}Glasses)) {
        if (position_meeting(mx, my, {ID}Glasses)) hover_glasses = true;
    }

    var hover_murray = false;
    if (mx >= (cx - 100) && mx <= (cx + 100) && my >= (cy - 15) && my <= (cy + 140)) hover_murray = true;
    if ({ID}Head != noone && instance_exists({ID}Head)) {
        if (position_meeting(mx, my, {ID}Head)) hover_murray = true;
    }
    if ({ID}Body != noone && instance_exists({ID}Body)) {
        if (position_meeting(mx, my, {ID}Body)) hover_murray = true;
    }

    var flashing = false;
    if (mouse_check_button(mb_right)) flashing = true;
    if (keyboard_check(vk_control) || keyboard_check(ord("F"))) flashing = true;
    if (variable_global_exists("Flashlight") && global.Flashlight) flashing = true;
    if (variable_global_exists("flashlight") && global.flashlight) flashing = true;

    // Quitar gafas: Clic izquierdo exclusivo en las gafas
    if (has_glasses && hover_glasses && mouse_check_button_pressed(mb_left)) {
        has_glasses = false;
        glasses_flying = true;
        glasses_vx = choose(-5, 5);
        glasses_vy = -7;
        remove_cooldown = 15;
        head_recoil_x = (glasses_vx > 0 ? 12 : -12);
        head_recoil_y = -6;

        var spr_scare = asset_get_index("s{ID}_head_Scare");
        if (spr_scare != -1 && {ID}Head != noone && instance_exists({ID}Head)) {
            {ID}Head.sprite_index = spr_scare;
        }

        var snd_click = asset_get_index("sfxClick");
        if (snd_click == -1) snd_click = asset_get_index("snd_Button");
        if (snd_click != -1) audio_play_sound(snd_click, 10, false);

        io_clear();
    }

    // Alumbrar a {ID} sin gafas: Derrota cómica, huida y la persiana se baja sola
    if (!has_glasses && flashing && remove_cooldown <= 0 && hover_murray) {
        state = 2; // Huida rápida
        blind_fade = 30;

        if ({ID}Body != noone && instance_exists({ID}Body)) {
            instance_destroy({ID}Body);
            {ID}Body = noone;
        }
        if ({ID}Glasses != noone && instance_exists({ID}Glasses)) {
            instance_destroy({ID}Glasses);
            {ID}Glasses = noone;
        }
        glasses_flying = false;

        // Liberar persiana para que caiga de inmediato automáticamente
        if (instance_exists(my_blind)) {
            my_blind.ForcedOpen = false;
            my_blind.Open = false;
        }

        var snd_bclose = asset_get_index("sfxBlindsClose");
        if (snd_bclose != -1) audio_play_sound(snd_bclose, 8, false);

        var spr_flash = asset_get_index("s{ID}_Flash");
        if ({ID}Head != noone && instance_exists({ID}Head)) {
            if (spr_flash != -1) {ID}Head.sprite_index = spr_flash;
            {ID}Head.x = cx - 94;
            {ID}Head.y = cy;
            {ID}Head.DrawDepth = 54;
        }

        var snd_hit = asset_get_index("sfxIgnitedFreddy_Hit1");
        if (snd_hit == -1) snd_hit = asset_get_index("sfxFlash");
        if (snd_hit != -1) audio_play_sound(snd_hit, 10, false);
    }

    // Si se agota el tiempo sin ahuyentarlo y la persiana está abierta -> Jumpscare
    if (attack_timer <= 0) {
        state = 4;
        jumpscare_timer = 0;
        jumpscare_frame = 0;
        glasses_flying = false;
        if ({ID}Body != noone && instance_exists({ID}Body)) { instance_destroy({ID}Body); {ID}Body = noone; }
        if ({ID}Head != noone && instance_exists({ID}Head)) { instance_destroy({ID}Head); {ID}Head = noone; }
        if ({ID}Glasses != noone && instance_exists({ID}Glasses)) { instance_destroy({ID}Glasses); {ID}Glasses = noone; }

        if (instance_exists(my_blind)) {
            my_blind.ForcedOpen = false;
        }

        audio_stop_all();

        // REPRODUCIR LA MÚSICA ALEGRE DEL JUMPSCARE ("tuuuun tunn tuuun")
        var _p_scare = "snd_{id}_jumpscare.ogg";
        if (!file_exists(_p_scare)) _p_scare = working_directory + "snd_{id}_jumpscare.ogg";
        if (!file_exists(_p_scare)) _p_scare = program_directory + "snd_{id}_jumpscare.ogg";

        if (file_exists(_p_scare)) {
            if (snd_jumpscare_stream != -1) {
                audio_destroy_stream(snd_jumpscare_stream);
                snd_jumpscare_stream = -1;
            }
            snd_jumpscare_stream = audio_create_stream(_p_scare);
            audio_play_sound(snd_jumpscare_stream, 100, false);
        } else {
            var snd_scream = asset_get_index("sfxJumpscareNormal");
            if (snd_scream == -1) snd_scream = asset_get_index("sfxWindowScare");
            if (snd_scream != -1) audio_play_sound(snd_scream, 10, false);
        }
    }
}

// 6. ESTADO 2: Huida cómica rápida hacia abajo
if (state == 2) {
    blind_fade--;
    if ({ID}Head != noone && instance_exists({ID}Head)) {
        {ID}Head.y += 3.5;
    }
    if (instance_exists(my_blind)) {
        my_blind.ForcedOpen = false;
        my_blind.Open = false;
    }
    if (blind_fade <= 0) {
        state = 0;
        var ai = 20;
        if (variable_global_exists("{ID}_AI")) ai = clamp(global.{ID}_AI, 0, 20);
        knock_timer = irandom_range(round(lerp(1800, 600, ai / 20.0)), round(lerp(2800, 1000, ai / 20.0)));
        if ({ID}Head != noone && instance_exists({ID}Head)) {
            instance_destroy({ID}Head);
            {ID}Head = noone;
        }
    }
}

// 7. ESTADO 4: Jumpscare -> Fijación de global.TimeOfDeath y paso a GameOver
if (state == 4) {
    jumpscare_timer++;
    jumpscare_frame = (floor(jumpscare_timer / 2) % 3);

    if (jumpscare_timer >= 120) {
        global.{ID}Death = true;
        global.Jumpscare = -1;

        global.TimeOfDeath = global.{ID}_SavedTimeSeconds;

        var _total_sec = floor(global.TimeOfDeath);
        var _mins = floor(_total_sec / 60);
        var _secs = _total_sec mod 60;
        var _millis = floor((global.TimeOfDeath - _total_sec) * 100);
        global.{ID}_SavedTime = string(_mins) + ":" + (_secs < 10 ? "0" : "") + string(_secs) + "." + (_millis < 10 ? "0" : "") + string(_millis);
        global.TimeLasted = global.{ID}_SavedTime;

        var r_go = asset_get_index("GameOver");
        if (r_go != -1) room_goto(r_go);
        else game_restart();
    }
}
"#;

/// oMurray_BackWindow -> Draw_64: jumpscare dinámico centrado con embestida,
/// sacudida de cámara y alternancia de fotogramas (golden V25).
const BW_DRAW64_TEMPLATE: &str = r#"// ==============================================================
// {ID} - Jumpscare Dinámico y Centrado (Draw GUI, V26-aligned)
// Solución al problema de altura: Desplazamiento vertical ajustado (+85.0 * scale)
// para que el hocico, ojos y mandíbula queden a la altura de los ojos del jugador.
// Generado por FrickEditor Studio (golden_master_v26)
// ==============================================================
if (state == 4) {
    var s_jump = asset_get_index("s{ID}_Jumpscare");
    if (s_jump != -1) {
        var gw = display_get_gui_width();
        var gh = display_get_gui_height();

        // Curva de embestida hacia la pantalla (de 0 a 1 en 28 frames)
        var lunge_t = clamp(jumpscare_timer / 28.0, 0.0, 1.0);
        var lunge_curve = power(lunge_t, 0.5);

        // Escala base: inicia en 1.05x y embiste hacia la cara del jugador (1.42x)
        var base_gui = max(gw / 640.0, gh / 360.0);
        var zoom_factor = lerp(1.05, 1.42, lunge_curve);
        var scale = base_gui * zoom_factor;

        var sw = 640.0 * scale;
        var sh = 640.0 * scale;

        // Sacudida de cámara violenta estilo FNaF / Frickbear's
        var shake_intensity = lerp(8.0, 18.0, lunge_curve) * (gw / 640.0);
        var shake_x = irandom_range(-round(shake_intensity), round(shake_intensity));
        var shake_y = irandom_range(-round(shake_intensity), round(shake_intensity));

        // Alternancia rápida de fotogramas de mordida (cada 2 frames)
        var sub_img = (floor(jumpscare_timer / 2) % 3);
        if (jumpscare_timer > 72) {
            sub_img = 2; // Mandíbula abierta al máximo en el impacto final antes de GameOver
            shake_intensity = 20.0 * (gw / 640.0);
            shake_x = irandom_range(-round(shake_intensity), round(shake_intensity));
            shake_y = irandom_range(-round(shake_intensity), round(shake_intensity));
        }

        // Posicionamiento horizontal y vertical perfecto (bajado para estar a la altura de los ojos)
        var draw_x = ((gw - sw) / 2.0) + shake_x;
        var draw_y = ((gh - sh) / 2.0) + (85.0 * scale) + shake_y;

        draw_sprite_ext(s_jump, sub_img, draw_x, draw_y, scale, scale, 0, c_white, 1.0);
    }
}
"#;

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
        let bad_render = addon_dir.join(format!("FB_{}_Moose.png", id));
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

        // 8. Copiar Audios (.ogg) a la carpeta del juego con el prefijo snd_ del protocolo
        //    (el GML generado los busca como snd_<id>_*.ogg, ver FRICKBEARS_ENGINE_REVERSE_ENGINEERING.md)
        let id_low = id.to_lowercase();
        Self::copy_audio(&proj.sound_knock, &proj.game_dir, &format!("snd_{}_knock.ogg", id_low))?;
        Self::copy_audio(&proj.sound_voice_arrive, &proj.game_dir, &format!("snd_{}_amigable.ogg", id_low))?;
        Self::copy_audio(&proj.sound_defeat, &proj.game_dir, &format!("snd_{}_jumpscare.ogg", id_low))?;

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
        let id_low = id.to_lowercase();
        let (tile_x, tile_y) = proj.mechanics.slot_to_xy();

        // Generar Create_0 del Tile en la posición exacta del Slot elegido
        // Constantes del botón PAGE 1/2 tomadas del golden_master_v26 (funciona en producción)
        let cn_create = format!(
            "// Generado por FrickEditor Studio — Slot #{slot}\n\
             depth = -1000;\n\
             if (!variable_global_exists(\"{id}_AI\")) global.{id}_AI = 0;\n\
             if (!variable_global_exists(\"CustomNight_Page\")) global.CustomNight_Page = 0;\n\
             btn_page_x = 344;\n\
             btn_page_y = 326;\n\
             btn_page_w = 110;\n\
             btn_page_h = 26;\n\
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

        // Generar Create_0 de la Oficina con los parámetros de IA y 4LD/Endless.
        // Reproduce la inicialización completa del golden_master_v26 (Invariante #2),
        // incluyendo tracking de tiempo de muerte y stream de jumpscare.
        let bw_create = format!(
            "// Generado por FrickEditor Studio — Invariante #2 Blindada\n\
             global.{id}Death = false;\n\
             {id}Body = noone;\n\
             {id}Head = noone;\n\
             {id}Glasses = noone;\n\
             survival_frames = 0;\n\
             global.{id}_SavedTimeSeconds = 0;\n\
             global.{id}_SavedTime = \"0:00.00\";\n\
             body_frame = 0;\n\
             snd_jumpscare_stream = -1;\n\
             var ai = 0;\n\
             if (variable_global_exists(\"{id}_AI\")) ai = clamp(global.{id}_AI, 0, 20);\n\
             if (variable_global_exists(\"{id}_Disabled\") && global.{id}_Disabled) ai = 0;\n\
             if (ai == 0 && variable_global_exists(\"EndlessMode\") && global.EndlessMode == true) ai = {endless_ai};\n\
             else if (ai == 0 && variable_global_exists(\"FourLayersDown\") && global.FourLayersDown == true) ai = {fld_ai};\n\
             if (ai <= 0) {{ instance_destroy(); exit; }}\n\
             state = 0;\n\
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

        // Step_0 del Tile: clics ◄ [00] ►, contador central, clic derecho, rueda del
        // ratón y botones globales aislados (SET ALL / ADD ALL) — golden_master_v26 V21
        let cn_step = Self::render_template(CN_STEP_TEMPLATE, id, &id_low);
        fs::write(
            gml_dir.join(format!("gml_Object_o{}_CustomNightTile_Step_0.gml", id)),
            cn_step,
        )
        .map_err(|e| e.to_string())?;

        // Draw_0 del Tile: cuadrícula nativa 8x4 + slot destacado + tooltip editable + botón PAGE
        let cn_draw = Self::render_template(CN_DRAW_TEMPLATE, id, &id_low).replace(
            "{tooltip}",
            &proj.mechanics.custom_night_tooltip.replace('"', "'"),
        );
        fs::write(
            gml_dir.join(format!("gml_Object_o{}_CustomNightTile_Draw_0.gml", id)),
            cn_draw,
        )
        .map_err(|e| e.to_string())?;

        // Step_0 del BackWindow: máquina de estados completa (0..4) — persianas,
        // gafas, Death Coin y jumpscare. Los offsets de las gafas vienen del proyecto.
        let bw_step = Self::render_template(BW_STEP_TEMPLATE, id, &id_low)
            .replace("{gx}", &proj.mechanics.glasses_offset_x.to_string())
            .replace("{gy}", &proj.mechanics.glasses_offset_y.to_string());
        fs::write(
            gml_dir.join(format!("gml_Object_o{}_BackWindow_Step_0.gml", id)),
            bw_step,
        )
        .map_err(|e| e.to_string())?;

        // Draw_64 del BackWindow: jumpscare dinámico centrado con embestida y sacudida
        let bw_draw64 = Self::render_template(BW_DRAW64_TEMPLATE, id, &id_low);
        fs::write(
            gml_dir.join(format!("gml_Object_o{}_BackWindow_Draw_64.gml", id)),
            bw_draw64,
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Sustituye los placeholders {ID} (identificador capitalizado) y {id} (en
    /// minúsculas) en las plantillas de eventos críticos del golden_master_v26.
    fn render_template(template: &str, id: &str, id_low: &str) -> String {
        template.replace("{ID}", id).replace("{id}", id_low)
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
            imageops::overlay(&mut canvas, head, 0, dy_head);

            let out_name = format!("s{}_Jumpscare_{}.png", id, idx);
            canvas
                .save(import_dir.join(out_name))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Comprueba que las plantillas quedan 100% parametrizadas para cualquier
    /// animatrónico custom y que ninguna referencia a "Murray" sobrevive al render.
    #[test]
    fn plantillas_se_parametrizan_con_el_id_del_proyecto() {
        let id = "Test";
        let id_low = "test";

        let cn_step = ProjectBuilder::render_template(CN_STEP_TEMPLATE, id, id_low);
        assert!(!cn_step.contains("{ID}") && !cn_step.contains("{id}"));
        assert!(cn_step.contains("global.Test_AI") && cn_step.contains("oCustomNightTile"));

        let cn_draw =
            ProjectBuilder::render_template(CN_DRAW_TEMPLATE, id, id_low).replace("{tooltip}", "Tip");
        assert!(!cn_draw.contains("{tooltip}"));
        assert!(cn_draw.contains("sTile_Test") && cn_draw.contains("global.Test_AI"));

        let bw_step = ProjectBuilder::render_template(BW_STEP_TEMPLATE, id, id_low)
            .replace("{gx}", "52")
            .replace("{gy}", "48");
        assert!(!bw_step.contains("{ID}") && !bw_step.contains("{id}"));
        assert!(!bw_step.contains("{gx}") && !bw_step.contains("{gy}"));
        assert!(bw_step.contains("TestBody") && bw_step.contains("global.Test_AI"));
        assert!(bw_step.contains("snd_test_knock.ogg") && bw_step.contains("snd_test_jumpscare.ogg"));

        let bw_draw64 = ProjectBuilder::render_template(BW_DRAW64_TEMPLATE, id, id_low);
        assert!(bw_draw64.contains("sTest_Jumpscare"));
        assert!(!bw_draw64.contains("Murray") && !bw_draw64.contains("murray"));
    }
}
