# FrickEditor Studio 🐻🛠️
**The Definitive Visual IDE, Mod Manager & Headless Injector for *Five Nights at Frickbear's 3 + Four Layers Down***

> **Platform:** Rust 2021 Edition (`eframe` / `egui 0.29`, `rodio` audio engine, `image`, `phosphor` icons)  
> **Output:** Compile-ready UndertaleModTool C# Automation Scripts (`.csx`) for direct, headless `data.win` injection and one-click play.

---

## 🚀 Purpose & Vision

**FrickEditor Studio** is an all-in-one visual modding environment engineered specifically for *Five Nights at Frickbear's 3 + Four Layers Down* (GameMaker LTS VM bytecode). It transforms what used to require manual UndertaleModTool (UTMT) decompilation and scripting into a professional, visual drag-and-drop experience.

Modders and developers can create animatronics, configure gallery profiles, design camera routes, place props in 3D mazes/salvage rooms, resolve slot collisions, and launch the modified game with **a single click**.

---

## 🎛️ The 7 Integrated IDE Modules

FrickEditor Studio is organized into 7 specialized workspaces:

### 0. Gestor de Mods & One-Click PLAY (`Tab 0`)
* **Mod Manager Dashboard:** Scans the `addons/` and workspace directory for installed `.fbmod` packages (Murray Moose, custom characters).
* **Slot Collision Detection & Auto-Resolution:** Automatically detects when two animatronics compete for the same Custom Night slot (1–32) and offers one-click intelligent re-indexing.
* **Headless Background Injection:** Launches UndertaleModTool in silent mode (`CREATE_NO_WINDOW = 0x08000000`) using CLI parameters or C# `UndertaleIO.Write` without displaying popup dialogs or freezing the UI.
* **One-Click PLAY:** Compiles all active mods into `master_injector.csx`, injects objects, sprites, sounds, and room props into `data.win`, and instantly boots `Frickbears 3.exe`.

### 1. Mod Gallery (`Tab 1`)
* Visual metadata editor for character name, author, developer trivia, hints, and shop descriptions.
* Generates standard `extras_info.txt` compatible with the in-game Gallery room.
* Enforces strict safety rules (prevents engine crashes caused by placing text folders into `skins/`).

### 2. Sprites & Audios (`Tab 2`)
* Full asset hub for importing body parts, accessories, flashlight stun states, and jumpscare frames.
* **Automatic Dimension Resizing:** Generates $500 \times 500$ portraits and $48 \times 48$ icons automatically.
* **Automatic Grayscale Tile:** Generates `sTile_1.png` (inactive / Death Coin state) using luminance weighting.
* **Real-time Audio Player:** Integrated `rodio` audio playback engine for previewing knocking alerts, voice lines, defeat sounds, and jumpscares directly inside the app.

### 3. Animación 60 FPS (`Tab 3`)
* Real-time interactive canvas running at 60 FPS.
* Allows previewing all 4 animatronic states:
  * **State 1:** Normal idle with sunglasses.
  * **State 1B:** Startled state (sunglasses knocked off).
  * **State 2:** Flashlight stun / defeat state.
  * **State 4:** Jumpscare sequence rendering.

### 4. Motor de Comportamiento e IA (`Tab 4`)
* **Custom Night Grid Manager:** Selector interactivo de casillas para la Página de Mods de Custom Night (32 slots) con cálculo automático y scroll seguro con `TreeY` (`base_y + rodney.TreeY`).
* **Sistema Híbrido de Nodos Visuales (`egui_node_graph`):** Permite diseñar la lógica del animatrónico conectando cables entre cajas de Eventos, Condiciones y Acciones. Traduce automáticamente el grafo visual a código GML VM seguro.
* **Sobrescritura Manual GML:** Editor de código integrado para programar directamente sobre los scripts del motor (`Step_0`, `Create_0`, `CleanUp_0`, `Draw_0`, `Draw_64`).
* **Simulador de Oficina a 60 FPS Integrado:** Visualizador interactivo en tiempo real con controles para subir/bajar persianas, activar linterna (F) y probar clics en accesorios, validando la regla invariante de seguridad #3.

### 5. Ruteo de Cámaras (`Tab 5`)
* **Room 33 Minimap Router:** Plano interactivo del restaurante con 9 cámaras (Cam 01 Dining Area a Cam 09 Stage).
* Multiplicadores configurables de velocidad de transición entre cámaras.
* Genera automáticamente los arrays GML `global.<ID>_CameraRoute = [...]`.

### 6. Constructor 3D (Laberintos, Conductos & Modo Endless) (`Tab 6`)
* **Lienzo Blueprint Drag & Drop:** Colocación visual de entidades interactivas para las salas de laberintos 3D (`MinigameMaze_1..5`), conductos de ventilación y arenas Endless.
* Soporte nativo para props del motor:
  * `oInteraction_Animatronic` (Punto de aparición / susto)
  * `oMazeFloorDecoration_Hatchet` (Hacha interactiva en suelo)
  * `oMazeProp_GiftBox` (Cajas de regalo y coleccionables)
  * `oMazeSign_Poster` (Pósters y lore ambiental)
  * `oMazeProp_ArcadeCabinet` (Máquinas recreativas)
  * `oInteraction_Escape` (Puerta de escape / salida)
* Inyecta instancias en los chunks de sala del motor en `data.win`.

---

## 📂 Repository Structure

```text
FrickEditor/
├── Cargo.toml                              # Dependencias Rust (eframe, egui, egui_node_graph2, rodio, image)
├── run.bat                                 # Lanzador rápido en 1 clic para Windows
├── src/
│   ├── main.rs                             # Ventana principal, navegación, simuladores y 7 módulos
│   ├── node_editor.rs                      # Motor de nodos visuales Drag & Drop (egui_node_graph) y traductor GML
│   ├── models.rs                           # Estructuras de datos, manifiestos, rutas y props
│   ├── builder.rs                          # Procesador de assets, generador de scripts y compilador CSX
│   ├── mod_manager.rs                      # Gestor de mods, detector y auto-resolución de slots
│   └── launcher.rs                         # Inyector headless de UTMT y ejecutor de Frickbears 3.exe
├── assets/
│   └── sample_murray/                      # Complete working sample assets (sprites, audio, icons)
├── templates/
│   ├── golden_master_v26/                  # Reference V26 CSX injector & GML scripts
│   └── golden_master_v27/                  # Production V27 CSX injector (safe TitleMenu hooks) & GML
├── docs/
│   ├── ARCHITECTURE_FRICKEDITOR_RUST.md    # IDE architecture specifications
│   ├── FRICKBEARS_ENGINE_REVERSE_ENG.md    # Engine bytecode reverse engineering reference
│   ├── frickbears_master_database.xml      # Master GameMaker database dictionary
│   ├── 00_INFORME_COMPLETO_PARA_ARAGON.md  # Technical report and engine invariants for Aragón
│   └── MURRAY_CODE_FOR_ARAGON.xml          # Complete Murray Moose GML codebase dump
└── schemas/
    └── character_manifest_schema.json      # Standard .fbmod package schema
```

---

## 🏗️ Quick Start & Build

### Opción 1: Lanzador Directo en Windows (Recomendado)
Haz doble clic en:
`run.bat`

### Opción 2: Desde Terminal con Cargo
```bash
# Compilar y correr en modo desarrollo
cargo run

# Compilar y correr en modo optimizado a 60 FPS
cargo run --release
```

El ejecutable directo compilado se encuentra en:
[frick_editor.exe](file:///D:/Frickbears-mod/FrickEditor_GitHub_Repository/target/debug/frick_editor.exe)

---

## 📜 Engine Invariant Rules (`GEMINI.md`)

When contributing scripts or mechanics, adhere strictly to the engine invariants:
1. **Strict Versioning:** Never overwrite past injection scripts (`V26`, `V27`, etc.). Always increment.
2. **Safe Lifecycle (Create & CleanUp):** Calling `instance_destroy()` in `Create_0` triggers `CleanUp_0` immediately. Initialize all instance variables at line 0 before any conditional destruction.
3. **Blinds & Window Safety:** Animatronics on the back window must **never** force blinds open (`my_blind.Open = true` is forbidden to protect against Sun/Moon instant deaths).
4. **Custom Night Grid Math:** Vanilla tiles calculate scroll with `draw_y = base_y + TreeY`. Validate mouse collisions against dynamic scroll coordinates.
5. **Addons vs. Skins Isolation:** Character metadata must live in `addons/<Name>/extras_info.txt`. Never create text folders in `skins/` (avoids `string_to_int_before_underscore` crashes).
