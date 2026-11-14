use crate::builder::IdeBuilder;
use crate::models::AnimatronicProject;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveGameProfile {
    Vanilla,
    Modded,
}

impl ActiveGameProfile {
    pub fn display_name(&self) -> &'static str {
        match self {
            ActiveGameProfile::Vanilla => "Vanilla Limpio (Sin Mods)",
            ActiveGameProfile::Modded => "Modded (Murray & Amigos)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub active_profile: String,
    pub last_switched: String,
    pub active_mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameIntegrityReport {
    pub game_dir: PathBuf,
    pub game_exe_path: Option<PathBuf>,
    pub game_exe_name: String,
    pub game_exe_size: u64,
    pub data_win_exists: bool,
    pub data_win_size: u64,
    pub vanilla_backup_exists: bool,
    pub vanilla_backup_path: Option<PathBuf>,
    pub vanilla_backup_size: u64,
    pub active_profile: ActiveGameProfile,
    pub utmt_exe_path: Option<PathBuf>,
    pub is_healthy: bool,
    pub details: Vec<String>,
}

fn simple_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02} UTC", hours, mins, s)
}

pub struct GameLauncher;

impl GameLauncher {
    /// Devuelve la ruta estándar para el respaldo vanilla intocable
    pub fn get_vanilla_backup_path(game_dir: &Path) -> PathBuf {
        game_dir.join("data_vanilla.win")
    }

    /// Busca si ya existe una copia de respaldo limpia previa en el directorio
    pub fn find_existing_backup_candidate(game_dir: &Path) -> Option<PathBuf> {
        let std_backup = Self::get_vanilla_backup_path(game_dir);
        if std_backup.exists() {
            return Some(std_backup);
        }

        let candidates = [
            "data_bak - copia - copia - copia.win",
            "data_bak.win",
            "data.win.bak",
            "data_backup.win",
            "data.win.vanilla",
        ];

        for name in candidates {
            let p = game_dir.join(name);
            if p.exists() {
                if let Ok(meta) = std::fs::metadata(&p) {
                    if meta.len() > 10_000_000 {
                        return Some(p);
                    }
                }
            }
        }

        None
    }

    /// Asegura que exista una copia de respaldo intacta (data_vanilla.win)
    pub fn ensure_vanilla_backup(game_dir: &Path) -> Result<PathBuf, String> {
        let target = Self::get_vanilla_backup_path(game_dir);
        if target.exists() {
            if let Ok(meta) = std::fs::metadata(&target) {
                if meta.len() > 10_000_000 {
                    return Ok(target);
                }
            }
        }

        // Si hay una copia previa limpia en el directorio, usarla preferentemente
        if let Some(candidate) = Self::find_existing_backup_candidate(game_dir) {
            if candidate != target {
                std::fs::copy(&candidate, &target)
                    .map_err(|e| format!("Error clonando respaldo desde {}: {}", candidate.display(), e))?;
                return Ok(target);
            }
        }

        // Si no hay respaldo previo pero existe data.win, crear el respaldo desde data.win
        let data_win = game_dir.join("data.win");
        if data_win.exists() {
            std::fs::copy(&data_win, &target)
                .map_err(|e| format!("Error creando respaldo de data.win: {}", e))?;
            return Ok(target);
        }

        Err("No se encontró data.win ni ninguna copia de seguridad en el directorio del juego.".to_string())
    }

    /// Fuerza la creación o renovación del respaldo intacto de data.win
    pub fn create_vanilla_backup(game_dir: &Path, force: bool) -> Result<PathBuf, String> {
        let target = Self::get_vanilla_backup_path(game_dir);
        if !force && target.exists() {
            return Ok(target);
        }

        // Preferir candidate antiguo si existe, o data.win
        let source = if let Some(cand) = Self::find_existing_backup_candidate(game_dir) {
            if cand != target { cand } else { game_dir.join("data.win") }
        } else {
            game_dir.join("data.win")
        };

        if !source.exists() {
            return Err("No se encontró archivo fuente para respaldar en el directorio del juego.".to_string());
        }

        std::fs::copy(&source, &target)
            .map_err(|e| format!("Error al escribir {}: {}", target.display(), e))?;

        Ok(target)
    }

    /// Restaura el archivo data.win original limpio desde el respaldo intacto
    pub fn restore_vanilla_data(game_dir: &Path) -> Result<PathBuf, String> {
        let backup = Self::ensure_vanilla_backup(game_dir)?;
        let data_win = game_dir.join("data.win");

        std::fs::copy(&backup, &data_win)
            .map_err(|e| format!("Error restaurando data.win desde el respaldo: {}", e))?;

        // Guardar metadata de perfil activo
        Self::save_profile_state(game_dir, ActiveGameProfile::Vanilla, &[]);

        Ok(data_win)
    }

    /// Guarda el estado del perfil en un archivo local `.frick_profile.json`
    pub fn save_profile_state(game_dir: &Path, profile: ActiveGameProfile, active_mods: &[String]) {
        let meta = ProfileMetadata {
            active_profile: match profile {
                ActiveGameProfile::Vanilla => "vanilla".to_string(),
                ActiveGameProfile::Modded => "modded".to_string(),
            },
            last_switched: simple_timestamp(),
            active_mods: active_mods.to_vec(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&meta) {
            let _ = std::fs::write(game_dir.join(".frick_profile.json"), json);
        }
    }

    /// Lee el estado del perfil actual
    pub fn get_profile_state(game_dir: &Path) -> ActiveGameProfile {
        let profile_file = game_dir.join(".frick_profile.json");
        if let Ok(content) = std::fs::read_to_string(profile_file) {
            if let Ok(meta) = serde_json::from_str::<ProfileMetadata>(&content) {
                if meta.active_profile.to_lowercase() == "vanilla" {
                    return ActiveGameProfile::Vanilla;
                } else if meta.active_profile.to_lowercase() == "modded" {
                    return ActiveGameProfile::Modded;
                }
            }
        }
        ActiveGameProfile::Vanilla
    }

    /// Realiza un chequeo exhaustivo de integridad del juego
    pub fn check_integrity(game_dir: &Path, utmt_scripts_dir: &Path) -> GameIntegrityReport {
        let mut details = Vec::new();
        let game_exe = Self::find_game_executable(game_dir);
        let mut game_exe_name = String::from("(No encontrado)");
        let mut game_exe_size = 0;

        if let Some(ref ge) = game_exe {
            game_exe_name = ge.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Ok(meta) = std::fs::metadata(ge) {
                game_exe_size = meta.len();
                details.push(format!("Ejecutable detectado: {} ({:.1} MB)", game_exe_name, game_exe_size as f64 / (1024.0 * 1024.0)));
            }
        } else {
            details.push("❌ ADVERTENCIA: No se localizó el ejecutable del juego en la carpeta.".to_string());
        }

        let data_win_path = game_dir.join("data.win");
        let data_win_exists = data_win_path.exists();
        let mut data_win_size = 0;
        if data_win_exists {
            if let Ok(meta) = std::fs::metadata(&data_win_path) {
                data_win_size = meta.len();
                details.push(format!("Archivo principal data.win: {:.1} MB ({} bytes)", data_win_size as f64 / (1024.0 * 1024.0), data_win_size));
            }
        } else {
            details.push("❌ ERROR CRÍTICO: No existe data.win en el directorio del juego.".to_string());
        }

        let backup_cand = Self::find_existing_backup_candidate(game_dir);
        let vanilla_backup_exists = backup_cand.is_some();
        let mut vanilla_backup_size = 0;
        if let Some(ref bp) = backup_cand {
            if let Ok(meta) = std::fs::metadata(bp) {
                vanilla_backup_size = meta.len();
                let b_name = bp.file_name().unwrap_or_default().to_string_lossy();
                details.push(format!("Copia de respaldo intacta: {} ({:.1} MB)", b_name, vanilla_backup_size as f64 / (1024.0 * 1024.0)));
            }
        } else {
            details.push("⚠️ Sin copia de seguridad: Se recomienda pulsar 'Generar Respaldo'.".to_string());
        }

        let utmt_exe = Self::find_utmt_executable(utmt_scripts_dir);
        if let Some(ref ue) = utmt_exe {
            let u_name = ue.file_name().unwrap_or_default().to_string_lossy();
            details.push(format!("UndertaleModTool listo: {}", u_name));
        } else {
            details.push("⚠️ UndertaleModTool no encontrado en rutas automáticas.".to_string());
        }

        let active_profile = Self::get_profile_state(game_dir);
        details.push(format!("Perfil activo actual: {}", active_profile.display_name()));

        let is_healthy = game_exe.is_some() && data_win_exists && (vanilla_backup_exists || data_win_size == 31020384);

        GameIntegrityReport {
            game_dir: game_dir.to_path_buf(),
            game_exe_path: game_exe,
            game_exe_name,
            game_exe_size,
            data_win_exists,
            data_win_size,
            vanilla_backup_exists,
            vanilla_backup_path: backup_cand,
            vanilla_backup_size,
            active_profile,
            utmt_exe_path: utmt_exe,
            is_healthy,
            details,
        }
    }

    /// Lanza el juego en Modo Vanilla Limpio:
    /// Restaura de forma instantánea data.win desde el respaldo intacto y ejecuta el juego oficial.
    pub fn launch_vanilla(game_dir: &Path) -> Result<String, String> {
        let game_exe = Self::find_game_executable(game_dir)
            .ok_or_else(|| format!("No se pudo encontrar el ejecutable del juego en {}", game_dir.display()))?;

        // 1. Restaurar data.win limpio
        Self::restore_vanilla_data(game_dir)?;

        // 2. Iniciar el juego limpio
        Self::launch_game(&game_exe)?;

        Ok(format!(
            "¡Modo Vanilla Activado! data.win restaurado al estado oficial limpio y juego iniciado: {}",
            game_exe.file_name().unwrap_or_default().to_string_lossy()
        ))
    }

    /// Inyecta el script maestro usando UndertaleModTool en modo silencioso (oculto en Windows)
    pub fn inject_silently(
        utmt_exe_path: &Path,
        data_win_path: &Path,
        master_csx_path: &Path,
    ) -> Result<String, String> {
        if !utmt_exe_path.exists() {
            return Err(format!(
                "No se encontró el ejecutable de UndertaleModTool en: {}",
                utmt_exe_path.display()
            ));
        }
        if !data_win_path.exists() {
            return Err(format!(
                "No se encontró el archivo data.win en: {}",
                data_win_path.display()
            ));
        }
        if !master_csx_path.exists() {
            return Err(format!(
                "No se encontró el script de inyección en: {}",
                master_csx_path.display()
            ));
        }

        let exe_name = utmt_exe_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let mut cmd = Command::new(utmt_exe_path);

        if exe_name.contains("cli") {
            // Sintaxis de UndertaleModCli (Modo Headless 100% silencioso)
            cmd.arg("load")
                .arg(data_win_path)
                .arg("-s")
                .arg(master_csx_path)
                .arg("-o")
                .arg(data_win_path);

            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);

            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

            let output = cmd
                .spawn()
                .map_err(|e| format!("Error al iniciar UndertaleModCli: {}", e))?
                .wait_with_output()
                .map_err(|e| format!("Error esperando a UndertaleModCli: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(format!(
                    "Inyección silenciosa completada con éxito.\n{}",
                    stdout.trim()
                ))
            } else {
                Err(format!(
                    "UndertaleModCli falló (código {:?}):\nSTDOUT: {}\nSTDERR: {}",
                    output.status.code(),
                    stdout.trim(),
                    stderr.trim()
                ))
            }
        } else {
            // UndertaleModTool GUI / SingleFile:
            cmd.arg(data_win_path);

            let _child = cmd
                .spawn()
                .map_err(|e| format!("Error al abrir UndertaleModTool: {}", e))?;

            Ok(format!(
                "UndertaleModTool abierto con data.win.\nScript maestro preparado en: {}\n\nEn la ventana de UTMT, haz clic en: Scripts -> master_injector.csx para compilar y guardar.",
                master_csx_path.display()
            ))
        }
    }

    /// Inicia el ejecutable del juego Frickbear's 3 en su directorio correspondiente
    pub fn launch_game(game_exe_path: &Path) -> Result<(), String> {
        if !game_exe_path.exists() {
            return Err(format!(
                "No se encontró el ejecutable del juego en: {}",
                game_exe_path.display()
            ));
        }

        let game_dir = game_exe_path.parent().unwrap_or_else(|| Path::new(""));

        Command::new(game_exe_path)
            .current_dir(game_dir)
            .spawn()
            .map_err(|e| format!("No se pudo iniciar el juego: {}", e))?;

        Ok(())
    }

    /// Localiza automáticamente el ejecutable del juego en el directorio configurado
    pub fn find_game_executable(game_dir: &Path) -> Option<PathBuf> {
        let candidates = [
            "Frickbears 3.exe",
            "FiveNightsAtFrickbears3.exe",
            "Frickbears3.exe",
            "Frickbear's 3.exe",
            "game.exe",
        ];

        for name in candidates {
            let p = game_dir.join(name);
            if p.exists() {
                return Some(p);
            }
        }

        // Si no coincide con los nombres exactos, busca cualquier .exe que no sea un instalador/uninstaller
        if let Ok(entries) = std::fs::read_dir(game_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("exe") {
                        let name_low = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        if !name_low.contains("unins") && !name_low.contains("setup") && !name_low.contains("modtool") {
                            return Some(p);
                        }
                    }
                }
            }
        }

        None
    }

    /// Localiza el ejecutable de UndertaleModTool (GUI o CLI)
    pub fn find_utmt_executable(utmt_scripts_dir: &Path) -> Option<PathBuf> {
        // 1. Probar en el directorio padre de Scripts
        if let Some(parent) = utmt_scripts_dir.parent() {
            let candidate = parent.join("UndertaleModTool.exe");
            if candidate.exists() {
                return Some(candidate);
            }
            let candidate_cli = parent.join("UndertaleModCli.exe");
            if candidate_cli.exists() {
                return Some(candidate_cli);
            }
        }

        // 2. Probar rutas conocidas del entorno
        let known = [
            PathBuf::from(r"D:\UndertaleModTool_v0.9.2.0-Windows-SingleFile\UndertaleModTool.exe"),
            PathBuf::from(r"C:\UndertaleModTool\UndertaleModTool.exe"),
        ];

        for p in known {
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    /// Flujo integral "One-Click Play" para Modo Modded:
    /// 1. Asegura la copia de seguridad intacta data_vanilla.win.
    /// 2. Restaura data.win limpio para garantizar que no haya corrupción previa.
    /// 3. Empaqueta y exporta todos los animatrónicos activos.
    /// 4. Genera el script maestro `master_injector.csx`.
    /// 5. Inyecta silenciosamente en `data.win`.
    /// 6. Inicia `Frickbears 3.exe`.
    pub fn build_inject_and_play(
        active_mods: &[AnimatronicProject],
        utmt_scripts_dir: &Path,
        game_dir: &Path,
    ) -> Result<String, String> {
        if active_mods.is_empty() {
            return Err("No hay animatrónicos activos seleccionados para inyectar.".to_string());
        }

        let data_win_path = game_dir.join("data.win");
        if !data_win_path.exists() {
            return Err(format!(
                "No se encontró 'data.win' en el directorio del juego: {}",
                game_dir.display()
            ));
        }

        let utmt_exe = Self::find_utmt_executable(utmt_scripts_dir)
            .ok_or_else(|| "No se pudo localizar UndertaleModTool.exe automáticamente.".to_string())?;

        let game_exe = Self::find_game_executable(game_dir)
            .ok_or_else(|| format!("No se pudo encontrar el ejecutable del juego en {}", game_dir.display()))?;

        // 1 & 2. Blindaje estilo Minecraft: Asegurar respaldo y restaurar base limpia antes de inyectar
        let _ = Self::ensure_vanilla_backup(game_dir)?;
        Self::restore_vanilla_data(game_dir)?;

        // 3. Construir todos los mods activos y generar `master_injector.csx`
        let master_csx_path = IdeBuilder::build_all_active_mods(active_mods, game_dir, utmt_scripts_dir, true)?;

        // 4. Inyección silenciosa o preparación en UTMT
        let inject_result = Self::inject_silently(&utmt_exe, &data_win_path, &master_csx_path)?;

        let mod_names: Vec<String> = active_mods.iter().map(|m| m.internal_id.clone()).collect();
        Self::save_profile_state(game_dir, ActiveGameProfile::Modded, &mod_names);

        let is_cli = utmt_exe
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .contains("cli");

        if is_cli {
            // Si es CLI, ya inyectó y guardó en data.win, arrancamos el juego inmediatamente
            Self::launch_game(&game_exe)?;
            Ok(format!(
                "¡Éxito! Inyectados {} mod(s) sobre base limpia y juego iniciado: {}",
                active_mods.len(),
                game_exe.file_name().unwrap_or_default().to_string_lossy()
            ))
        } else {
            // Si es GUI, UTMT se abrió con data.win cargado
            Ok(format!(
                "¡Base limpia preparada y Script Maestro Generado!\n{}\n(Tras ejecutarlo en UTMT, puedes abrir el juego directamente).",
                inject_result
            ))
        }
    }

    /// Abre UndertaleModTool directamente cargando el archivo data.win del juego
    pub fn open_undertale_mod_tool(utmt_scripts_dir: &Path, game_dir: &Path) -> Result<String, String> {
        let utmt_exe = Self::find_utmt_executable(utmt_scripts_dir)
            .ok_or_else(|| "No se encontró UndertaleModTool.exe en las rutas configuradas (ej. D:\\UndertaleModTool_v0.9.2.0-Windows-SingleFile\\UndertaleModTool.exe)".to_string())?;

        let data_win_path = game_dir.join("data.win");
        if !data_win_path.exists() {
            return Err(format!("No se encontró 'data.win' en el directorio del juego: {}", game_dir.display()));
        }

        let mut cmd = Command::new(&utmt_exe);
        if let Some(parent) = utmt_exe.parent() {
            cmd.current_dir(parent);
        }
        cmd.arg(&data_win_path);

        match cmd.spawn() {
            Ok(_) => Ok(format!("UndertaleModTool ejecutado con: {}", data_win_path.display())),
            Err(e) => Err(format!("Error iniciando UndertaleModTool: {}", e)),
        }
    }

    /// Abre el directorio de scripts de UTMT en el explorador de archivos
    pub fn open_utmt_scripts_folder(utmt_scripts_dir: &Path) -> Result<(), String> {
        if !utmt_scripts_dir.exists() {
            let _ = std::fs::create_dir_all(utmt_scripts_dir);
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(utmt_scripts_dir)
                .spawn()
                .map_err(|e| format!("Error abriendo carpeta de scripts: {}", e))?;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("xdg-open")
                .arg(utmt_scripts_dir)
                .spawn()
                .map_err(|e| format!("Error abriendo carpeta: {}", e))?;
            Ok(())
        }
    }

    /// Abre el directorio de instalación del juego en el explorador de archivos
    pub fn open_game_folder(game_dir: &Path) -> Result<(), String> {
        if !game_dir.exists() {
            return Err(format!("No existe el directorio del juego: {}", game_dir.display()));
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(game_dir)
                .spawn()
                .map_err(|e| format!("Error abriendo carpeta del juego: {}", e))?;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("xdg-open")
                .arg(game_dir)
                .spawn()
                .map_err(|e| format!("Error abriendo carpeta: {}", e))?;
            Ok(())
        }
    }

    /// Abre la nota de auditoría técnica en el editor predeterminado (o Notepad en Windows)
    pub fn open_audit_note(workspace_dir: &Path) -> Result<(), String> {
        let candidates = [
            workspace_dir.join("FrickEditor_GitHub_Repository").join("docs").join("AUDITORIA_MODIFICACIONES_GAME.md"),
            workspace_dir.join("docs").join("AUDITORIA_MODIFICACIONES_GAME.md"),
            workspace_dir.join("AUDITORIA_MODIFICACIONES_GAME.md"),
        ];

        for p in candidates {
            if p.exists() {
                #[cfg(target_os = "windows")]
                {
                    Command::new("notepad.exe")
                        .arg(&p)
                        .spawn()
                        .map_err(|e| format!("Error abriendo nota de auditoría: {}", e))?;
                    return Ok(());
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Command::new("xdg-open")
                        .arg(&p)
                        .spawn()
                        .map_err(|e| format!("Error abriendo nota de auditoría: {}", e))?;
                    return Ok(());
                }
            }
        }
        Err("No se encontró el archivo AUDITORIA_MODIFICACIONES_GAME.md".to_string())
    }
}
