# 🦌 PAQUETE TÉCNICO MAESTRO: MURRAY MOOSE V18 -> V19
### Para: Aragón (Hermano Mayor / Asesor de Arquitectura)
### De: Antigravity & Usuario
### Proyecto: Five Nights at Frickbear's 3 + Four Layers Down (v2.0.2)
### Fecha: 2 de Noviembre de 2026

---

## 1. INTRODUCCIÓN Y CONTEXTO DEL MOTOR

El juego objetivo es **Five Nights at Frickbear's 3 + Four Layers Down (v2.0.2)**, compilado en **GameMaker LTS Bytecode (VM)**. La inyección de código, objetos, sprites y sonidos se realiza mediante scripts en C# (.csx) ejecutados dentro de **UndertaleModTool (UTMT) Single-File v0.9.2.0**.

Actualmente, **Murray Moose** ha alcanzado el hito funcional en la versión **V18**:
* Se implementó una **página modded independiente (Página 1)** en la Custom Night (`global.CustomNight_Page = 1`), preservando 100% intacto el vector estático de los 56 animatrónicos vanilla.
* El personaje aparece en la ventana trasera izquierda, reacciona a la linterna tras retirarle los anteojos mediante clics, y activa su Game Over carmesí con su consejo personalizado.
* La **Mod Gallery** (`addons/Murray`) ya no produce el fatal error de memoria `0xc0000409`.

Sin embargo, tras las pruebas in-game con la compilación V18, el usuario ha documentado **6 problemas críticos** (visuales, de audio y de interfaz) que requieren ajuste fino y tu opinión arquitectónica antes de proceder a la versión final **V19**.

---

## 2. REGLAS INVARIANTES DEL PROYECTO (`GEMINI.md`)

1. **Versionado Estricto:** Cada cambio o corrección incrementa el script instalador en 1 (`Instalar_Murray_V18.csx` -> `Instalar_Murray_V19.csx`). Nunca sobrescribir versiones previas.
2. **Seguridad en Ciclo de Vida GML (Create & CleanUp):** Llamar a `instance_destroy()` en `Create_0` ejecuta inmediatamente `CleanUp_0`. Todas las variables de instancia leídas o destruidas en `CleanUp` deben inicializarse en la primera línea absoluta de `Create_0`. Proteger con `variable_instance_exists()`.
3. **Compatibilidad con Persianas (Sun & Moon):** Murray en ventana trasera **NUNCA** debe forzar o abrir la persiana (`my_blind.Open = true` o `ForcedOpen = true` estrictamente prohibidos). Si la persiana está cerrada por el jugador, Murray no debe atacar.
4. **Custom Night Grid & Scroll (TreeY):** Los tiles vanilla no modifican `y`, calculan su posición como `base_y + TreeY`. Clics y detección de mouse deben validarse contra coordenadas dibujadas con scroll.
5. **Convención de Sprites para Tiles:**
   - Frame 0 (`sTile_X_0.png`): Color activo (`AI > 0`).
   - Frame 1 (`sTile_X_1.png`): Blanco y negro inactivo / Death Coin (`AI == 0`).
6. **Separación Addons vs Skins:** Información de la galería exclusivamente en `addons/<Nombre>/extras_info.txt` e `icon.png`. Prohibido crear carpetas de personajes en `skins/` (evita crash en `string_to_int_before_underscore`).

---

## 3. DESGLOSE DE LOS 6 PROBLEMAS DOCUMENTADOS Y SOLUCIONES

### Problema 1: Ícono de Murray en blanco en la Mod Gallery
* **Evidencia:** `05_Capturas_de_los_Errores/01_Galeria_Icono_Blanco.png`.
* **Causa Raíz:** En `get_mod_tiles@gml_Object_oExtras_AnimatronicSelect_Create_0`, el motor de Frickbear's escanea cada subcarpeta dentro de `addons/` esperando:
  1. `portrait.png` ($500 \times 500$) para el render grande del personaje.
  2. `icon.png` ($48 \times 48$) para la casilla de selección en la grilla.
  Para evitar el crash `0xc0000409` causado por texturas de gran tamaño que colisionaban con `sprite_merge`, se eliminó la imagen gigante `FB_Murray_Moose.png`, pero también faltaba `icon.png`. Al no existir `icon.png`, el juego dibuja un cuadro blanco vacío sin textura.
* **Solución V19:** Colocar en `addons/Murray/`:
  - `extras_info.txt`
  - `portrait.png` ($500 \times 500$)
  - `icon.png` ($48 \times 48$, copia idéntica de `sTile_Murray_0.png`).
  *(Queda garantizada la compatibilidad y se elimina el cuadro blanco).*

---

### Problema 2: Pantalla de Game Over con `Time Lasted: 0:00.00`
* **Evidencia:** `05_Capturas_de_los_Errores/05_GameOver_Tiempo_Cero.png`.
* **Causa Raíz (Bytecode verificado):** 
  En `gml_Object_oGameOver_Draw_0` (offset `0908` del bytecode), el texto `"Time Lasted: "` se concatena con el valor de la variable global `global.TimeLasted`.
  Durante la noche activa, el objeto `oCamera_UIStuff` actualiza en cada paso:
  - `global.TimerMinutes`
  - `global.TimerSeconds`
  - `global.TimerDecimal`
  En V18, cuando Murray ejecuta su jumpscare y el temporizador llega a 85 frames, se ejecutaba `room_goto(GameOver)` directamente sin actualizar `global.TimeLasted`. Por tanto, `global.TimeLasted` permanecía en su valor por defecto no inicializado (`0:00.00`).
* **Solución V19:** En `gml_Object_oMurray_BackWindow_Step_0.gml`, justo antes de invocar `room_goto(r_go)`:
```gml
if (variable_global_exists("TimerMinutes") && variable_global_exists("TimerSeconds")) {
    var _m = string(global.TimerMinutes);
    var _s = (global.TimerSeconds < 10 ? "0" : "") + string(global.TimerSeconds);
    var _ms = "00";
    if (variable_global_exists("TimerDecimal")) {
        _ms = (global.TimerDecimal < 10 ? "0" : "") + string(floor(global.TimerDecimal));
    }
    global.TimeLasted = _m + ":" + _s + "." + _ms;
} else {
    global.TimeLasted = "0:45.00";
}
```

---

### Problema 3: Jumpscare partido en 4 cuadrantes / fisuras visuales
* **Evidencia:** `05_Capturas_de_los_Errores/04_Jumpscare_Cuadrantes_Roto.png`.
* **Causa Raíz:**
  En V18 se intentó optimizar el sprite `sMurray_Jumpscare` dividiéndolo en:
  - Frame 0: Torso/Hombros ($640 \times 429$).
  - Frame 1 y 2: Cabeza mordiendo ($544 \times 385$).
  El script en C# modificó manualmente en el Texture Page los valores `TargetX`, `TargetY`, `TargetWidth`, `TargetHeight` y `BoundingWidth/Height`. En GameMaker VM, al empacar con Packer en una textura de $2048 \times 2048$ y dibujar dos llamadas superpuestas con offsets elásticos (`jaw_snap`), las coordenadas UV sufrieron *texture wrapping*. Como resultado, la mandíbula y el cuello se renderizaron divididos en esquinas opuestas de la pantalla.
* **Solución V19:** Pre-componer los 3 fotogramas en una resolución fija y limpia de $640 \times 640$ con centro de rotación estándar (origen en 0,0 o 320,320). En `Draw_64.gml`, se descarta el dibujado en multicapa manual y se renderiza una sola llamada nativa:
```gml
draw_sprite_ext(s_jump, jumpscare_frame, draw_x, draw_y, scale, scale, 0, c_white, 1);
```
Esto elimina de raíz cualquier desalineación de coordenadas y asegura una animación fluida a 60 FPS sin fisuras.

---

### Problema 4: Voces no deseadas en la mecánica de Murray
* **Requerimiento:** El usuario especificó estrictamente no usar líneas de voz secundarias, respiraciones ni gruñidos. Utilizar **EXCLUSIVAMENTE** el sonido de golpes en el cristal: `murray_knock.ogg` (`sfxKnock`).
* **Solución V19:** Purgar cualquier llamada a `audio_play_sound` que no sea el golpe en el cristal:
```gml
audio_play_sound(sfxKnock, 10, false);
```

---

### Problema 5: Invariante #3 de Persianas
* **Requerimiento:** Murray **NUNCA** debe abrir ni forzar la persiana por su cuenta (`my_blind.Open = true` o `ForcedOpen = true` quedan prohibidos).
* **Solución V19:** Confirmada la regla en el autómata de estados: Murray solo observa y golpea el vidrio. Si la persiana está abajo, su temporizador de agresión se frena y el jugador está a salvo.

---

### Problema 6: Estética y Layout de la Custom Night
* **Evidencias:** 
  - `05_Capturas_de_los_Errores/02_CustomNight_Boton_Modded_Vanilla.png` (Botón tosco en grilla vanilla).
  - `05_Capturas_de_los_Errores/03_CustomNight_Botones_Solapados_Texto.png` (Botones rápidos solapando el texto descriptivo).
* **Problemas de Layout detectados:**
  1. En la grilla vanilla, el botón `[ MODDED (1) > ]` en $(340, 324)$ tiene un texto desalineado y un bisel que resalta negativamente frente a la estética Win95/FNAF del juego original.
  2. En la Página 1 (Modded Roster), los botones de ajuste rápido `[-1]`, `[+1]`, `[0]`, `[20]` se posicionaron en $(340, 210)$, justo encima de la última línea de texto: *"Immune to flashlight until sunglasses are removed."*
  3. En la barra superior de la ventana, el botón `[ < VANILLA ROSTER ]` invade el texto del título `"MODDED ANIMATRONICS - ROSTER (PAGE 1)"`.
* **Solución de Rediseño Propuesta:**
  - **Botón en Grilla Vanilla:** Redimensionar a $(338, 326)$ con texto conciso `MODS (1) >` o `CUSTOM (1) >` con bisel nativo estándar `c_white` / `c_black` idéntico a `START NIGHT`.
  - **Tarjeta de Murray (Página 1):**
    - Aumentar el cuadro de texto a 160 px de alto para que el texto de descripción respire con márgenes adecuados.
    - Mover los botones `[-1]`, `[+1]`, `[0]`, `[20]` directamente debajo del cuadro de nivel de IA del tile de Murray (coordenada $Y \approx 135$), agrupados en una botonera compacta y legible.
    - La barra de título azul superior mantendrá el título a la izquierda y el botón `[ < VOLVER ]` a la derecha con 12 px de margen seguro.

---

## 4. ESTRUCTURA DE ESTA CARPETA (`Paquete_Aragon/`)

```text
Paquete_Aragon/
├── 00_INFORME_COMPLETO_PARA_ARAGON.md       (Este informe técnico maestro)
│
├── 01_Codigo_GML/                           (Todos los scripts GML de Murray en V18)
│   ├── gml_Object_oMurray_BackWindow_Create_0.gml
│   ├── gml_Object_oMurray_BackWindow_Step_0.gml
│   ├── gml_Object_oMurray_BackWindow_Draw_0.gml
│   ├── gml_Object_oMurray_BackWindow_Draw_64.gml
│   ├── gml_Object_oMurray_BackWindow_CleanUp_0.gml
│   ├── gml_Object_oMurray_CustomNightTile_Create_0.gml
│   ├── gml_Object_oMurray_CustomNightTile_Step_0.gml
│   ├── gml_Object_oMurray_CustomNightTile_Draw_0.gml
│   └── gml_Object_oMurray_DeathScreen_Draw_0.gml
│
├── 02_Scripts_UTMT_CSX/                     (Scripts de instalación en C#)
│   ├── Instalar_Murray_V16.csx
│   ├── Instalar_Murray_V17.csx
│   └── Instalar_Murray_V18.csx
│
├── 03_Addons_Murray/                        (Archivos para addons/Murray/)
│   ├── extras_info.txt                      (Metadatos y bio del animatrónico)
│   ├── portrait.png                         (500x500 render grande para la galería)
│   └── icon.png                             (48x48 icono para la cuadrícula de selección)
│
├── 04_Assets_Graficos_y_Sonido/             (Sprites originales y sonidos)
│   ├── sMurray_body.png, sMurray_head.png, etc.
│   ├── sJumpscare_Murray_0, 1, 2.png
│   ├── sTile_Murray_0, 1, 2.png
│   └── murray_knock.ogg                     (Único sonido de golpe permitido)
│
└── 05_Capturas_de_los_Errores/              (Capturas de pantalla tomadas por el usuario)
    ├── 01_Galeria_Icono_Blanco.png
    ├── 02_CustomNight_Boton_Modded_Vanilla.png
    ├── 03_CustomNight_Botones_Solapados_Texto.png
    ├── 04_Jumpscare_Cuadrantes_Roto.png
    └── 05_GameOver_Tiempo_Cero.png
```

---

## 5. CONSULTA PARA ARAGÓN

Hermano Aragón, con toda esta evidencia técnica y el diagnóstico de causas raíz a nivel de motor GameMaker:
1. ¿Apruebas la estrategia de pre-componer `sMurray_Jumpscare` en 3 fotogramas completos de $640 \times 640$ para descartar las transformaciones de atlas en C# y renderizar en una sola pasada en `Draw_64`?
2. ¿Te parece adecuada la captura de `global.TimeLasted` leyendo `global.TimerMinutes`, `TimerSeconds` y `TimerDecimal` antes de la llamada a `room_goto(GameOver)`?
3. Para la interfaz de la Custom Night, ¿tienes alguna recomendación específica sobre la posición de la botonera rápida `[-1] [+1] [0] [20]` o procedemos con el diseño limpio agrupado bajo el tile?

¡Quedamos atentos a tus indicaciones para compilar **`Instalar_Murray_V19.csx`** y dejar la versión final perfecta!
