// ==============================================================
// Murray Moose - Custom Night Tile Logic (Step V21)
// Cuadrícula nativa, flechas perfectas (0..16, 16..32, 32..48) y botones globales aislados
// ==============================================================
depth = -1000;
if (!variable_global_exists("Murray_AI")) global.Murray_AI = 0;
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
    // B. Controles de Murray en Página 2 (Slot 0,0 en 32, 12)
    // ◄ en 32..47 (0..15), Contador en 48..63 (16..31), ► en 64..79 (32..47)
    else if (global.CustomNight_Page == 1) {
        if (mouse_y >= cur_tile_y + 48 && mouse_y <= cur_tile_y + 64) {
            // Flecha Izquierda ◄ (Decrementar)
            if (mouse_x >= cur_tile_x && mouse_x < cur_tile_x + 16) {
                if (global.Murray_AI > 0) {
                    global.Murray_AI--;
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                }
            }
            // Flecha Derecha ► (Incrementar)
            else if (mouse_x >= cur_tile_x + 32 && mouse_x <= cur_tile_x + 48) {
                if (global.Murray_AI < 20) {
                    global.Murray_AI++;
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                }
            }
            // Clic en el número central (Alternar 0 / 20)
            else if (mouse_x >= cur_tile_x + 16 && mouse_x < cur_tile_x + 32) {
                global.Murray_AI = (global.Murray_AI > 0) ? 0 : 20;
                if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
            }
        }
        
        // C. Botones globales del panel derecho (SET ALL 0, ADD ALL 1, ADD ALL 5, SET ALL 20)
        // EXCLUSIVO de la Página 2: Solo afecta a Murray y no toca a los vanilla
        if (mouse_x >= 478 && mouse_x <= 612) {
            // Fila 1: SET ALL 0 y ADD ALL 1 (Y: 35..71)
            if (mouse_y >= 35 && mouse_y <= 71) {
                if (mouse_x < 545) {
                    global.Murray_AI = 0; // SET ALL 0
                    if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
                } else {
                    global.Murray_AI = min(20, global.Murray_AI + 1); // ADD ALL 1
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                }
            }
            // Fila 2: ADD ALL 5 y SET ALL 20 (Y: 72..108)
            else if (mouse_y >= 72 && mouse_y <= 108) {
                if (mouse_x < 545) {
                    global.Murray_AI = min(20, global.Murray_AI + 5); // ADD ALL 5
                    if (snd_click != -1) audio_play_sound(snd_click, 0, false);
                } else {
                    global.Murray_AI = 20; // SET ALL 20
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

// 2. Clic derecho en el Tile de Murray para alternar 0 / 20
if (global.CustomNight_Page == 1 && mouse_check_button_pressed(mb_right)) {
    if (mouse_x >= cur_tile_x && mouse_x <= cur_tile_x + 48 && mouse_y >= cur_tile_y && mouse_y <= cur_tile_y + 64) {
        global.Murray_AI = (global.Murray_AI > 0) ? 0 : 20;
        if (snd_btn != -1) audio_play_sound(snd_btn, 0, false);
    }
}

// 3. Rueda del ratón encima del Tile de Murray
if (global.CustomNight_Page == 1) {
    if (mouse_x >= cur_tile_x && mouse_x <= cur_tile_x + 48 && mouse_y >= cur_tile_y && mouse_y <= cur_tile_y + 64) {
        if (mouse_wheel_up()) {
            if (global.Murray_AI < 20) {
                global.Murray_AI++;
                if (snd_click != -1) audio_play_sound(snd_click, 0, false);
            }
        } else if (mouse_wheel_down()) {
            if (global.Murray_AI > 0) {
                global.Murray_AI--;
                if (snd_click != -1) audio_play_sound(snd_click, 0, false);
            }
        }
    }
}
