if (variable_global_exists("MurrayDeath") && global.MurrayDeath) {
    if (variable_global_exists("Murray_SavedTimeSeconds") && global.Murray_SavedTimeSeconds > 0) {
        global.TimeOfDeath = global.Murray_SavedTimeSeconds;
    }
    if (variable_global_exists("Murray_SavedTime")) {
        global.TimeLasted = global.Murray_SavedTime;
    }
}
