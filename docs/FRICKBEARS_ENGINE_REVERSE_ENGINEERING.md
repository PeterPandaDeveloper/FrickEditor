# Five Nights at Frickbear's 3 — Reverse Engineering Specifications

This document outlines the exact internal architecture and runtime invariants of *Five Nights at Frickbear's 3 + Four Layers Down* (GameMaker Studio 2.3+ bytecode runtime). Any code generator or visual editor targeting this game **must** strictly adhere to these rules.

---

## 1. Custom Night Grid & Scrolling Architecture (`TreeY`)
* **Grid Layout:** Custom Night tiles (`oCustomNightTile`) are arranged in an $8 \times 4$ grid ($32$ slots per page).
* **Scroll Mechanics:** Unlike standard objects, tiles do **not** modify their instance `y` variable on scroll. Instead, screen drawing and mouse collision detection use:
  $$\text{draw\_x} = \text{rodney.x} + 48$$
  $$\text{draw\_y} = \text{base\_y} + \text{TreeY}$$
  Where `TreeY` is the menu scroll offset (`rodney.TreeY` or `oCustomNightMenu.TreeY`).
* **Tile Dimension & Texture Convention:**
  - Tile dimensions are exactly $48 \times 48\text{ px}$.
  - **Frame 0 (`sTile_<Name>_0.png`):** Full color when selected ($AI > 0$).
  - **Frame 1 (`sTile_<Name>_1.png`):** Grayscale when not selected ($AI == 0$) or when disabled via Death Coin (`DeathCoined = true`).
  - Native arrows use `sCustomNightArrows` ($32\text{ px}$ wide: $16\text{ px}$ left, $16\text{ px}$ right) placed alongside counter `sCustomNightCounter` ($16\text{ px}$ wide) at $x + 32$.

---

## 2. Window Blinds & Sun/Moon Coexistence Invariants
* **Window Coordinates:**
  - Left Office Window: $(X: 160, Y: 1085)$.
  - Right Office Window: $(X: 1462, Y: 1085)$.
* **Persianas / Blinds Invariant:**
  - An animatronic at a window **must never** force a blind open (`my_blind.ForcedOpen = true` or `my_blind.Open = true`) autonomously.
  - Reason: If the central poster is Sun, forcing blinds open causes instantaneous player death by Sun.
  - **Graceful Blind Protocol:** If the user raises the blind, the animatronic can hold it open (`my_blind.ForcedOpen = true`). As soon as the animatronic is scared away (e.g., flashed with light), the blind must drop automatically (`my_blind.ForcedOpen = false; my_blind.Open = false;`) accompanied by `sfxBlindsClose`.

---

## 3. GameMaker Audio Sandboxing & File Paths
* In GameMaker Studio VM running on Windows:
  - Relative paths like `file_exists("snd_scare.ogg")` check `%LOCALAPPDATA%\Frickbears3\` first.
  - If a file is only copied to the game installation folder (`C:\Juegos\...`), `file_exists` without a full directory path **will fail**.
* **Audio Protocol:**
  - Audio files (`.ogg`) must be deployed to **both**:
    1. Game executable folder (`<game_dir>\snd_<name>.ogg`).
    2. LocalAppData sandbox (`%LOCALAPPDATA%\Frickbears3\snd_<name>.ogg`).
  - GML code must resolve with fallbacks:
    ```gml
    var p = "snd_scare.ogg";
    if (!file_exists(p)) p = working_directory + "snd_scare.ogg";
    if (!file_exists(p)) p = program_directory + "snd_scare.ogg";
    if (file_exists(p)) {
        var stream = audio_create_stream(p);
        audio_play_sound(stream, 100, false);
    }
    ```

---

## 4. Game Over Lifecycle & Voiceline Suppression
* When killed by a custom animatronic:
  1. Freeze survival time in `global.TimeOfDeath = global.SavedTimeSeconds`.
  2. Set `global.MurrayDeath = true` (or custom character flag) and `global.Jumpscare = -1`.
  3. Transition to `room_goto(GameOver)`.
* **In `gml_Object_oGameOver_Create_0`:**
  - In vanilla, `global.Jumpscare = -1` triggers a fallback random character voiceline (Salvage, Phone Guy, etc.).
  - To prevent unintended voicelines on death, inject:
    ```gml
    if (variable_global_exists("CustomDeath") && global.CustomDeath) {
        Tip = "Custom Character: Hint on how to survive.";
        audio_stop_all(); // Suppresses random voicelines
        // Plays standard defeat fanfare
        var mus = asset_get_index("musTitleScreen_Evil");
        if (mus != -1) audio_play_sound(mus, 10, false);
    }
    ```

---

## 5. In-Game Pause Menu Hook (`oTitleMenu` Review Animatronics)
* When the user presses `ESC`, `oPauser` spawns `oTitleMenu`.
* **DO NOT** modify `oPauser` or `oPersistentPauser` (doing so corrupts the keyboard capture for `ESC`).
* Instead, hook `gml_Object_oTitleMenu_Step_0`:
  - Detect `_in_review`: when `Options` contains `"Back"` and does not contain `"Resume"` or settings.
  - Insert `"Custom Character"` right before `"Back"`.
* Hook `gml_Object_oTitleMenu_Draw_77`:
  - When `Options[Select] == "Custom Character"`, render character description at $(X: 76, Y: 305)$ using `draw_text_ext`.

---

## 6. Strict Separation: Addons vs. Skins
* Character gallery metadata resides **exclusively** in:
  `addons/<CharacterName>/extras_info.txt` and `icon.png`.
* **NEVER** place character profiles in `skins/<Name>`, because the native script `string_to_int_before_underscore` parses skin folder prefixes as integers (`0_Freddy`), crashing on non-numeric folder names.
