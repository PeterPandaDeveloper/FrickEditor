use crate::builder::IdeBuilder;
use crate::models::AnimatronicProject;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct GameLauncher;

impl GameLauncher {
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
            // No pasar CREATE_NO_WINDOW ni -q porque WPF revienta al ocultar ventana sin consola.
            // Abrimos UndertaleModTool con data.win cargado y el script listo en Scripts/.
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

    /// Flujo integral "One-Click Play":
    /// 1. Empaqueta y exporta todos los animatrónicos activos.
    /// 2. Genera el script maestro `master_injector.csx`.
    /// 3. Inyecta silenciosamente en `data.win`.
    /// 4. Inicia `Frickbears 3.exe`.
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

        // 1 & 2. Construir todos los mods activos y generar `master_injector.csx`
        let master_csx_path = IdeBuilder::build_all_active_mods(active_mods, game_dir, utmt_scripts_dir, true)?;

        // 3. Inyección silenciosa o preparación en UTMT
        let inject_result = Self::inject_silently(&utmt_exe, &data_win_path, &master_csx_path)?;

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
                "¡Éxito! Inyectados {} mod(s) y juego iniciado: {}",
                active_mods.len(),
                game_exe.file_name().unwrap_or_default().to_string_lossy()
            ))
        } else {
            // Si es GUI, UTMT se abrió con data.win cargado
            Ok(format!(
                "¡Script Maestro Generado!\n{}\n(Tras ejecutarlo en UTMT, puedes abrir el juego directamente).",
                inject_result
            ))
        }
    }
}
