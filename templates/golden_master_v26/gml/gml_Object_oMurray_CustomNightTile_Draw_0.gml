// ==============================================================
// Murray Moose - Custom Night Tile Render (Draw V22)
// Cuadrícula estilizada (8x4 slots), ◄ [00] ► nativo y botón PAGE 1/2 alineado
// ==============================================================
var fnt = asset_get_index("fnt_Small");
if (fnt == -1) fnt = asset_get_index("fnt_Dialogue");
if (fnt != -1) draw_set_font(fnt);

// 1. Si estamos en la Página 2 (Murray Moose), dibujar la cuadrícula completa de 8x4
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
                // SLOT (0,0): MURRAY MOOSE
                if (spr_base != -1) {
                    draw_sprite(spr_base, 0, sx, sy);
                } else {
                    draw_set_color(make_color_rgb(192, 192, 192));
                    draw_rectangle(sx, sy, sx + 47, sy + 47, false);
                }
                
                // Retrato de Murray
                var spr_murray = asset_get_index("sTile_Murray");
                if (spr_murray != -1) {
                    var is_dc = (variable_global_exists("Murray_DeathCoined") && global.Murray_DeathCoined);
                    if (global.Murray_AI > 0 && !is_dc) {
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
                    draw_sprite(spr_counter, global.Murray_AI, sx + 16, sy + 48);
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
    
    // Tooltip oficial Windows 95 al pasar el cursor sobre Murray
    var show_desc = true;
    if (variable_global_exists("ShowDescriptions")) show_desc = global.ShowDescriptions;
    
    if (show_desc && mouse_x >= 32 && mouse_x <= 80 && mouse_y >= 12 && mouse_y <= 76) {
        var tip_txt = "Murray Moose: He knocks on the right office window. Raise the window blind to spot him, click his sunglasses to knock them off, and shine your flashlight to scare him away.";
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
