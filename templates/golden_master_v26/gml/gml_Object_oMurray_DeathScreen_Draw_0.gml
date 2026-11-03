// Pantalla de Game Over Oficial (Grim Foxy Style)
if (room == asset_get_index("GameOver")) {
    if (variable_global_exists("MurrayDeath") && global.MurrayDeath) {
        if (variable_global_exists("Murray_SavedTimeSeconds") && global.Murray_SavedTimeSeconds > 0) {
            global.TimeOfDeath = global.Murray_SavedTimeSeconds;
        }
        with (oGameOver) {
            Tip = "Murray Moose: Flashing him only works after you remove his sunglasses.";
        }
    }
}
