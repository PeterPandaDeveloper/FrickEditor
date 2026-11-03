// ==============================================================
// Murray Moose - Jumpscare Dinámico y Centrado (Draw GUI V25)
// Solución al problema de altura: Desplazamiento vertical ajustado (+85.0 * scale)
// para que el hocico, ojos y mandíbula queden a la altura de los ojos del jugador.
// ==============================================================
if (state == 4) {
    var s_jump = asset_get_index("sMurray_Jumpscare");
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
