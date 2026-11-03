// ==============================================================
// Murray Moose - Back Window Animatronic (Create Event V24)
// Cumple Invariante #2 (Inicialización absoluta en primeras líneas), tracking de tiempo y audio de jumpscare
// ==============================================================
global.MurrayDeath = false;
MurrayBody = noone;
MurrayHead = noone;
MurrayGlasses = noone;
survival_frames = 0;
global.Murray_SavedTimeSeconds = 0;
global.Murray_SavedTime = "0:00.00";
body_frame = 0;
snd_jumpscare_stream = -1;

var ai = 0;
if (variable_global_exists("Murray_AI")) {
    ai = clamp(global.Murray_AI, 0, 20);
}
if (variable_global_exists("Murray_Disabled") && global.Murray_Disabled) {
    ai = 0;
}

// Soporte para Four Layers Down (4LD) y Modo Endless si no viene de Custom Night
if (ai == 0 && variable_global_exists("EndlessMode") && global.EndlessMode == true) {
    ai = 10;
} else if (ai == 0 && variable_global_exists("FourLayersDown") && global.FourLayersDown == true) {
    ai = 12;
}

if (ai <= 0) {
    instance_destroy();
    exit;
}

state = 0;
attack_timer = round(lerp(600, 320, ai / 20.0));
knock_timer = irandom_range(round(lerp(480, 180, ai / 20.0)), round(lerp(900, 360, ai / 20.0)));
has_glasses = true;
glasses_flying = false;
glasses_vx = 0;
glasses_vy = 0;
blind_fade = 0;
jumpscare_timer = 0;
jumpscare_frame = 0;
remove_cooldown = 0;
head_recoil_x = 0;
head_recoil_y = 0;
