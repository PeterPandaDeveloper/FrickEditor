# FrickEditor: Enrutamiento Visual y Control de Código

Este documento rige cómo la interfaz de usuario en Rust debe leer y modificar el motor del juego sin romper el binario `data.win`.

## 1. Módulos del IDE y su Mapeo al XML

### A. Editor de Animatrónicos de Oficina (Core)
*   **Lectura:** `<AnimatronicProfiles>` y `<CustomNightRoster>`.
*   **Interfaz:** Sliders de IA (1-20), asignación de Slots en la cuadrícula (0-49), y mapeo de bounding boxes (`TargetX`, `TargetY`) para evitar colisiones en `sprite_merge`.
*   **Inyección:** `Instalar_<ID>_V<X>.csx` que parchea `CODE` y `OBJT` en la sala 3 (`Office`).

### B. Editor de Salvamentos (Salvage Room)
*   **Lectura:** `<SalvageMechanics>`.
*   **Interfaz:** Panel de simulación de tensión. El usuario sube los 3 sprites (Fase 1, 2, 3), configura el `base_tension` y el `aggression_multiplier`.
*   **Inyección:** El IDE genera `gml_Object_oSalvageEnemy_Step_0.gml` y lo inyecta en la sala 18 (`SalvageResults`), enlazando los archivos de audio del magnetófono y los sustos.

### C. Editor de Minijuegos y Eventos (Candy Cadet / Cutscenes)
*   **Lectura:** `<MinigamesAndEvents>`.
*   **Interfaz:** Línea de tiempo visual (Timeline) para cutscenes. Selector de probabilidades (RNG) para Candy Cadet (Tokens vs Historias).
*   **Inyección:** C# parchea `chunk_SOND` para los audios narrativos (.ogg) y sobreescribe las alarmas en `gml_Object_oMidnightCutscene_Alarm_<X>.gml`.

## 2. Invariantes de Seguridad del IDE
1.  **Cero Hardcoding de Strings:** Todo diálogo, descripción de galería (`extras_info.txt`) o Tooltip de Custom Night debe pasar por el codificador UTF-8 nativo del IDE antes de escribirse en disco.
2.  **Protección de Variables Globales:** El IDE debe envolver automáticamente toda inyección de variables externas (ej. `global.EndlessTokens`) con `if (variable_global_exists("X"))` para evitar crasheos si el mod se ejecuta en un *save file* vanilla.
3.  **Prevención de Memory Leaks en Audio:** Cualquier archivo `.ogg` subido para Candy Cadet o Salvamento debe registrarse mediante `audio_create_stream` en el evento `Create` y eliminarse con `audio_destroy_stream` en el evento `CleanUp` del objeto correspondiente.
