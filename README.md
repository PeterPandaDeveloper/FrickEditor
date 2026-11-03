# FrickEditor (FrickIDE) 🐻🛠️
**Visual IDE & Modding Studio for *Five Nights at Frickbear's 3 + Four Layers Down***

> **Target Language:** Rust (`eframe` / `egui` or `iced` + `wgpu`)  
> **Output:** Compile-ready UndertaleModTool C# Automation Scripts (`.csx`) for direct `data.win` injection.

---

## 🚀 Purpose & Vision
FrickEditor is a visual modding IDE built in Rust to make creating, configuring, and testing content for *Five Nights at Frickbear's 3* completely visual and intuitive:
1. **Character Creator & Asset Hub:** Upload sprites, animated strips (000..005), jumpscares, and audio files.
2. **Visual Office & Hitbox Workspace:** Drag and position animatronics directly in the office viewport (windows, vents, doors), set sunglasses/flashlight hitboxes, and test blind interactions visually.
3. **MIT Scratch / Block-Based Logic:** Node/Block-based behavior editor (GameMaker D&D style) for beginners, with direct GML/C# code view for advanced modders.
4. **Gallery & Salvage Studio:** Automatic configuration of `addons/<Name>/extras_info.txt`, custom night icons, salvage dialogue tapes, and shop items.
5. **1-Click CSX Compiler:** Automatically generates mathematically perfect UndertaleModTool `.csx` scripts that pack texture sheets, compile GameMaker VM code, inject room instances, and manage audio sandboxes.

---

## 📂 Repository Contents (For AI Collaborators / Aragón)

* **`docs/ARCHITECTURE_FRICKEDITOR_RUST.md`**: High-level and low-level architecture of the Rust app, UI modules, crate structure, block-coding VM, and `.csx` code generator.
* **`docs/FRICKBEARS_ENGINE_REVERSE_ENGINEERING.md`**: Complete game engine specifications reverse-engineered from vanilla GameMaker bytecode (grid scroll math, blind state flags, audio sandboxing, `oTitleMenu` pause hooks, game over lifecycle).
* **`schemas/character_manifest_schema.json`**: Standard JSON schema for `.frickmod` project files.
* **`templates/golden_master_v26/`**: The battle-tested working reference implementation (`Instalar_Murray_V26.csx` + GML scripts) that proves every single injection mechanic works in production.

---

## 🏗️ Quick Start for Development
```bash
# Clone the repository
git clone https://github.com/<your-username>/FrickEditor.git
cd FrickEditor

# Run with cargo (once crate initialized)
cargo run --release
```
