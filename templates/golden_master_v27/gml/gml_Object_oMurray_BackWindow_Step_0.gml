// ==============================================================
// Murray Moose - Back Window Logic (Step Event V25)
// Ventana derecha (1462, 1085), animación de cuerpo (frames 0..5),
// audio único de golpe snd_murray_knock.ogg, audio alegre de jumpscare snd_murray_jumpscare.ogg,
// y mecánica elegante de persiana: Si el usuario la levanta se queda arriba; al ahuyentarlo cae sola.
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
global.Murray_SavedTimeSeconds = cur_sec;

// 2. Verificación de Death Coin
var deathcoined = false;
if (variable_global_exists("Murray_DeathCoined") && global.Murray_DeathCoined) deathcoined = true;
if (variable_global_exists("Murray_Disabled") && global.Murray_Disabled) deathcoined = true;
if (variable_global_exists("DeathCoin_Target") && global.DeathCoin_Target == "Murray") deathcoined = true;
if (state == 1 && mouse_check_button_pressed(mb_left)) {
    if (variable_global_exists("HeldItem") && (global.HeldItem == "DeathCoin" || global.HeldItem == "Death Coin" || global.HeldItem == "oShopItem_DeathCoin")) {
        deathcoined = true;
    }
}

if (deathcoined) {
    if (MurrayBody != noone && instance_exists(MurrayBody)) { instance_destroy(MurrayBody); MurrayBody = noone; }
    if (MurrayHead != noone && instance_exists(MurrayHead)) { instance_destroy(MurrayHead); MurrayHead = noone; }
    if (MurrayGlasses != noone && instance_exists(MurrayGlasses)) { instance_destroy(MurrayGlasses); MurrayGlasses = noone; }
    glasses_flying = false;
    if (instance_exists(my_blind)) {
        my_blind.ForcedOpen = false;
        my_blind.Open = false;
    }
    var snd_dc = asset_get_index("sfxDeathCoin");
    if (snd_dc == -1) snd_dc = asset_get_index("sfxIgnitedFreddy_Hit1");
    if (snd_dc != -1) audio_play_sound(snd_dc, 10, false);
    global.Murray_DeathCoined = true;
    global.Murray_Disabled = true;
    instance_destroy();
    exit;
}

// 3. Física de las gafas al salir disparadas
if (glasses_flying && MurrayGlasses != noone && instance_exists(MurrayGlasses)) {
    MurrayGlasses.x += glasses_vx;
    MurrayGlasses.y += glasses_vy;
    glasses_vy += 1.2;
    if (MurrayGlasses.y > 1300) {
        instance_destroy(MurrayGlasses);
        MurrayGlasses = noone;
        glasses_flying = false;
    }
}

// 4. ESTADO 0: Fuera de la ventana trasera derecha
if (state == 0) {
    if (MurrayBody != noone && instance_exists(MurrayBody)) { instance_destroy(MurrayBody); MurrayBody = noone; }
    if (MurrayHead != noone && instance_exists(MurrayHead)) { instance_destroy(MurrayHead); MurrayHead = noone; }
    if (MurrayGlasses != noone && instance_exists(MurrayGlasses)) { instance_destroy(MurrayGlasses); MurrayGlasses = noone; }
    glasses_flying = false;
    
    knock_timer--;
    if (knock_timer <= 0) {
        // Murray llega a la ventana derecha (1462, 1085)
        state = 1;
        var ai = 20;
        if (variable_global_exists("Murray_AI")) ai = clamp(global.Murray_AI, 0, 20);
        attack_timer = round(lerp(600, 320, ai / 20.0));
        has_glasses = true;
        remove_cooldown = 0;
        head_recoil_x = 0;
        head_recoil_y = 0;
        body_frame = 0;
        
        var spr_body = asset_get_index("sMurray_body");
        var spr_head = asset_get_index("sMurray_head");
        var spr_glasses = asset_get_index("sMurray_Glasses");
        
        // El cuerpo animado centrado exactamente en la ventana
        MurrayBody = instance_create_layer(cx - 200, cy - 23, "Behind_Wall", oOfficeSprite);
        if (spr_body != -1) {
            MurrayBody.sprite_index = spr_body;
            MurrayBody.image_index = 0;
            MurrayBody.image_speed = 0;
        }
        MurrayBody.DrawDepth = 48;
        
        MurrayHead = instance_create_layer(cx - 77, cy, "Behind_Wall", oOfficeSprite);
        if (spr_head != -1) MurrayHead.sprite_index = spr_head;
        MurrayHead.DrawDepth = 50;
        
        MurrayGlasses = instance_create_layer(cx - 77 + 52, cy + 48, "Behind_Wall", oOfficeSprite);
        if (spr_glasses != -1) MurrayGlasses.sprite_index = spr_glasses;
        MurrayGlasses.DrawDepth = 52;
        
        // Al llegar, la persiana NO se fuerza abierta automáticamente:
        // si está cerrada, se mantiene cerrada para respetar la seguridad con Sun & Moon
        if (instance_exists(my_blind)) {
            my_blind.ForcedOpen = false;
        }
        
        // ÚNICO SONIDO PERMITIDO AL APARECER: snd_murray_knock.ogg
        var _p_knock = "snd_murray_knock.ogg";
        if (!file_exists(_p_knock)) _p_knock = working_directory + "snd_murray_knock.ogg";
        if (!file_exists(_p_knock)) _p_knock = program_directory + "snd_murray_knock.ogg";
        if (file_exists(_p_knock)) {
            var str_k = audio_create_stream(_p_knock);
            audio_play_sound(str_k, 10, false);
        } else {
            var snd_k = asset_get_index("snd_murray_knock");
            if (snd_k == -1) snd_k = asset_get_index("sfxKnock");
            if (snd_k != -1) audio_play_sound(snd_k, 10, false);
        }
    }
}

// 5. ESTADO 1: Murray acechando en la ventana derecha
if (state == 1) {
    // MECÁNICA ELEGANTE DE PERSIANA:
    // Si la persiana está cerrada: Murray NO la fuerza abierta y no puede atacar (timer pausado).
    // Si el usuario la levanta: Murray la sostiene alzada firmemente (ForcedOpen = true).
    if (instance_exists(my_blind)) {
        if (!my_blind.Open) {
            attack_timer = max(attack_timer, 180);
            my_blind.ForcedOpen = false;
        } else {
            // El usuario levantó la persiana: Murray la sostiene alzada cómodamente
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
    if (MurrayBody != noone && instance_exists(MurrayBody)) {
        if (body_frame < 5) {
            body_frame += 0.25;
            if (body_frame > 5) body_frame = 5;
        }
        MurrayBody.image_index = floor(body_frame);
        MurrayBody.image_speed = 0;
        MurrayBody.x = cx - 200;
        MurrayBody.y = (cy - 23) + torso_bob;
    }
    
    var head_bob = sin((current_time * 0.003) + 0.5) * 2.5;
    if (MurrayHead != noone && instance_exists(MurrayHead)) {
        MurrayHead.x = (cx - 77) + head_recoil_x;
        MurrayHead.y = cy + head_bob + head_recoil_y;
    }
    
    if (has_glasses && !glasses_flying && MurrayGlasses != noone && instance_exists(MurrayGlasses)) {
        MurrayGlasses.x = (cx - 77 + 52) + head_recoil_x;
        MurrayGlasses.y = (cy + 48) + head_bob + head_recoil_y;
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
    if (MurrayGlasses != noone && instance_exists(MurrayGlasses)) {
        if (position_meeting(mx, my, MurrayGlasses)) hover_glasses = true;
    }
    
    var hover_murray = false;
    if (mx >= (cx - 100) && mx <= (cx + 100) && my >= (cy - 15) && my <= (cy + 140)) hover_murray = true;
    if (MurrayHead != noone && instance_exists(MurrayHead)) {
        if (position_meeting(mx, my, MurrayHead)) hover_murray = true;
    }
    if (MurrayBody != noone && instance_exists(MurrayBody)) {
        if (position_meeting(mx, my, MurrayBody)) hover_murray = true;
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
        
        var spr_scare = asset_get_index("sMurray_head_Scare");
        if (spr_scare != -1 && MurrayHead != noone && instance_exists(MurrayHead)) {
            MurrayHead.sprite_index = spr_scare;
        }
        
        var snd_click = asset_get_index("sfxClick");
        if (snd_click == -1) snd_click = asset_get_index("snd_Button");
        if (snd_click != -1) audio_play_sound(snd_click, 10, false);
        
        io_clear();
    }
    
    // Alumbrar a Murray sin gafas: Derrota cómica, huida y la persiana se baja sola
    if (!has_glasses && flashing && remove_cooldown <= 0 && hover_murray) {
        state = 2; // Huida rápida
        blind_fade = 30;
        
        if (MurrayBody != noone && instance_exists(MurrayBody)) {
            instance_destroy(MurrayBody);
            MurrayBody = noone;
        }
        if (MurrayGlasses != noone && instance_exists(MurrayGlasses)) {
            instance_destroy(MurrayGlasses);
            MurrayGlasses = noone;
        }
        glasses_flying = false;
        
        // Liberar persiana para que caiga de inmediato automáticamente
        if (instance_exists(my_blind)) {
            my_blind.ForcedOpen = false;
            my_blind.Open = false;
        }
        
        var snd_bclose = asset_get_index("sfxBlindsClose");
        if (snd_bclose != -1) audio_play_sound(snd_bclose, 8, false);
        
        var spr_flash = asset_get_index("sMurray_Flash");
        if (MurrayHead != noone && instance_exists(MurrayHead)) {
            if (spr_flash != -1) MurrayHead.sprite_index = spr_flash;
            MurrayHead.x = cx - 94;
            MurrayHead.y = cy;
            MurrayHead.DrawDepth = 54;
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
        if (MurrayBody != noone && instance_exists(MurrayBody)) { instance_destroy(MurrayBody); MurrayBody = noone; }
        if (MurrayHead != noone && instance_exists(MurrayHead)) { instance_destroy(MurrayHead); MurrayHead = noone; }
        if (MurrayGlasses != noone && instance_exists(MurrayGlasses)) { instance_destroy(MurrayGlasses); MurrayGlasses = noone; }
        
        if (instance_exists(my_blind)) {
            my_blind.ForcedOpen = false;
        }
        
        audio_stop_all();
        
        // REPRODUCIR LA MÚSICA ALEGRE DEL JUMPSCARE ("tuuuun tunn tuuun")
        var _p_scare = "snd_murray_jumpscare.ogg";
        if (!file_exists(_p_scare)) _p_scare = working_directory + "snd_murray_jumpscare.ogg";
        if (!file_exists(_p_scare)) _p_scare = program_directory + "snd_murray_jumpscare.ogg";
        
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
    if (MurrayHead != noone && instance_exists(MurrayHead)) {
        MurrayHead.y += 3.5;
    }
    if (instance_exists(my_blind)) {
        my_blind.ForcedOpen = false;
        my_blind.Open = false;
    }
    if (blind_fade <= 0) {
        state = 0;
        var ai = 20;
        if (variable_global_exists("Murray_AI")) ai = clamp(global.Murray_AI, 0, 20);
        knock_timer = irandom_range(round(lerp(1800, 600, ai / 20.0)), round(lerp(2800, 1000, ai / 20.0)));
        if (MurrayHead != noone && instance_exists(MurrayHead)) {
            instance_destroy(MurrayHead);
            MurrayHead = noone;
        }
    }
}

// 7. ESTADO 4: Jumpscare -> Fijación de global.TimeOfDeath y paso a GameOver
if (state == 4) {
    jumpscare_timer++;
    jumpscare_frame = (floor(jumpscare_timer / 2) % 3);
    
    if (jumpscare_timer >= 120) {
        global.MurrayDeath = true;
        global.Jumpscare = -1;
        
        global.TimeOfDeath = global.Murray_SavedTimeSeconds;
        
        var _total_sec = floor(global.TimeOfDeath);
        var _mins = floor(_total_sec / 60);
        var _secs = _total_sec mod 60;
        var _millis = floor((global.TimeOfDeath - _total_sec) * 100);
        global.Murray_SavedTime = string(_mins) + ":" + (_secs < 10 ? "0" : "") + string(_secs) + "." + (_millis < 10 ? "0" : "") + string(_millis);
        global.TimeLasted = global.Murray_SavedTime;
        
        var r_go = asset_get_index("GameOver");
        if (r_go != -1) room_goto(r_go);
        else game_restart();
    }
}
