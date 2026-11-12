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

### 4. Cuadrícula & IA (`Tab 4`)
* **Custom Night Grid Manager:** Interactive selector for Custom Night Page 2 (56 modded slots) with automatic `TreeY` scroll-safe coordinate computation (`base_y + rodney.TreeY`).
* **Hybrid Logic Architecture:**
  * **Visual Nodes:** Automatically generates GameMaker Language (GML) code from high-level behavioral parameters.
  * **Manual GML Override:** Advanced modders can write custom GML directly into an integrated code editor with syntax protection.

### 5. Ruteo de Cámaras (`Tab 5`)
* **Room 33 Minimap Router:** Interactive floor plan of the restaurant with 9 camera waypoints (Cam 01 Dining Area to Cam 09 Stage).
* Configurable transition speed multipliers for each waypoint.
* Generates `global.<ID>_CameraRoute = [...]` GameMaker initialization arrays automatically.

### 6. Constructor 3D de Laberintos & Salvamento (`Tab 6`)
* **Blueprint Drag & Drop Canvas:** Visual 2D/3D prop placement for `MinigameMaze_1..5` and `Room 18 (Salvage Room)`.
* Support for engine props:
  * `oInteraction_Animatronic` (Spawn point)
  * `oMazeFloorDecoration_Hatchet` (Hatchet weapon)
  * `oMazeProp_GiftBox` (Collectibles)
  * `oMazeSign_Poster` (Environmental lore)
  * `oMazeProp_ArcadeCabinet` (Mini-game triggers)
  * `oInteraction_Escape` (Exit door)
* Injects room chunk instances directly into GameMaker room data.

---

## 📂 Repository Structure

```text
FrickEditor/
├── Cargo.toml                              # Rust dependencies (eframe, egui, rodio, image, serde)
├── src/
│   ├── main.rs                             # Main UI, tabs, canvases, 60fps animation preview
│   ├── models.rs                           # Data structures, manifests, camera routes, maze props
│   ├── builder.rs                          # Asset processor, GML generator, master CSX builder
│   ├── mod_manager.rs                      # Mod manager, slot conflict detector & auto-resolver
│   └── launcher.rs                         # Headless UTMT background runner & Game process launcher
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

### Requirements
* [Rust](https://www.rust-lang.org/) (1.75+ recommended with Cargo)
* Windows 10/11 (for UndertaleModTool execution and GameMaker VM target)

### Build & Run
```bash
# Clone the repository
git clone https://github.com/PeterPandaDeveloper/FrickEditor.git
cd FrickEditor

# Run development build
cargo run

# Build optimized release binary
cargo build --release
```

The compiled binary will be located at:
`target/release/frick_editor.exe`

---

## 📜 Engine Invariant Rules (`GEMINI.md`)

When contributing scripts or mechanics, adhere strictly to the engine invariants:
1. **Strict Versioning:** Never overwrite past injection scripts (`V26`, `V27`, etc.). Always increment.
2. **Safe Lifecycle (Create & CleanUp):** Calling `instance_destroy()` in `Create_0` triggers `CleanUp_0` immediately. Initialize all instance variables at line 0 before any conditional destruction.
3. **Blinds & Window Safety:** Animatronics on the back window must **never** force blinds open (`my_blind.Open = true` is forbidden to protect against Sun/Moon instant deaths).
4. **Custom Night Grid Math:** Vanilla tiles calculate scroll with `draw_y = base_y + TreeY`. Validate mouse collisions against dynamic scroll coordinates.
5. **Addons vs. Skins Isolation:** Character metadata must live in `addons/<Name>/extras_info.txt`. Never create text folders in `skins/` (avoids `string_to_int_before_underscore` crashes).
