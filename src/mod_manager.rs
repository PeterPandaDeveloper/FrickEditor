use crate::builder::IdeBuilder;
use crate::models::AnimatronicProject;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ModManifest {
    pub active_mods: HashMap<String, bool>,
}

pub struct ModManager {
    pub installed_mods: Vec<AnimatronicProject>,
    pub active_mods: HashMap<String, bool>,
    pub mods_dir: PathBuf,
}

impl ModManager {
    pub fn new(mods_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&mods_dir);
        let mut mgr = Self {
            installed_mods: Vec::new(),
            active_mods: HashMap::new(),
            mods_dir,
        };
        mgr.load_manifest();
        mgr
    }

    /// Carga el estado de activación guardado previamente
    pub fn load_manifest(&mut self) {
        let manifest_file = self.mods_dir.join("mods_manifest.json");
        if manifest_file.exists() {
            if let Ok(data) = fs::read_to_string(&manifest_file) {
                if let Ok(m) = serde_json::from_str::<ModManifest>(&data) {
                    self.active_mods = m.active_mods;
                }
            }
        }
    }

    /// Guarda el estado de activación en el disco
    pub fn save_manifest(&self) {
        let manifest_file = self.mods_dir.join("mods_manifest.json");
        let manifest = ModManifest {
            active_mods: self.active_mods.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(manifest_file, json);
        }
    }

    /// Escanea la carpeta `mods_dir` en busca de archivos `.fbmod` y `.json`
    pub fn scan_mods(&mut self, workspace_dir: &Path) -> usize {
        let _ = fs::create_dir_all(&self.mods_dir);
        let mut count = 0;

        if let Ok(entries) = fs::read_dir(&self.mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("fbmod") || ext.eq_ignore_ascii_case("zip") {
                        if let Ok(proj) = IdeBuilder::import_fbmod_package(&path, workspace_dir) {
                            self.add_or_update_mod(proj);
                            count += 1;
                        }
                    } else if ext.eq_ignore_ascii_case("json") && !path.file_name().unwrap_or_default().to_string_lossy().contains("manifest") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(proj) = serde_json::from_str::<AnimatronicProject>(&content) {
                                self.add_or_update_mod(proj);
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        // Si la lista está vacía, aseguramos al menos el mod predeterminado Murray Moose
        if self.installed_mods.is_empty() {
            let default_proj = AnimatronicProject::default();
            self.add_or_update_mod(default_proj);
        }

        self.save_manifest();
        count
    }

    /// Agrega un proyecto nuevo o actualiza uno existente por internal_id
    pub fn add_or_update_mod(&mut self, proj: AnimatronicProject) {
        let id = proj.internal_id.clone();
        if let Some(existing) = self.installed_mods.iter_mut().find(|m| m.internal_id == id) {
            *existing = proj;
        } else {
            if !self.active_mods.contains_key(&id) {
                self.active_mods.insert(id.clone(), true);
            }
            self.installed_mods.push(proj);
        }
    }

    /// Importa un archivo `.fbmod` desde una ruta arbitraria hacia la carpeta del gestor
    pub fn import_fbmod(&mut self, fbmod_path: &Path, workspace_dir: &Path) -> Result<AnimatronicProject, String> {
        let proj = IdeBuilder::import_fbmod_package(fbmod_path, workspace_dir)?;
        let dest = self.mods_dir.join(format!("{}.fbmod", proj.internal_id));
        if fbmod_path != dest {
            let _ = fs::copy(fbmod_path, &dest);
        }
        self.add_or_update_mod(proj.clone());
        self.active_mods.insert(proj.internal_id.clone(), true);
        self.save_manifest();
        Ok(proj)
    }

    /// Consulta si un mod está activo
    pub fn is_active(&self, id: &str) -> bool {
        *self.active_mods.get(id).unwrap_or(&false)
    }

    /// Alterna el estado activo/inactivo de un animatrónico
    pub fn set_active(&mut self, id: &str, active: bool) {
        self.active_mods.insert(id.to_string(), active);
        self.save_manifest();
    }

    /// Obtiene referencias a todos los proyectos activos
    pub fn get_active_mods(&self) -> Vec<AnimatronicProject> {
        self.installed_mods
            .iter()
            .filter(|m| self.is_active(&m.internal_id))
            .cloned()
            .collect()
    }

    /// Revisa si hay conflictos (ej. dos mods intentan usar el mismo Slot de Custom Night)
    pub fn check_conflicts(&self) -> Vec<String> {
        let mut conflicts = Vec::new();
        let mut used_slots: HashMap<usize, String> = HashMap::new();

        for proj in &self.installed_mods {
            if self.is_active(&proj.internal_id) {
                let slot = proj.custom_night.slot_index;
                if let Some(other_mod) = used_slots.insert(slot, proj.internal_id.clone()) {
                    conflicts.push(format!(
                        "¡Conflicto en Custom Night! '{}' y '{}' intentan usar el Slot #{} (Casilla {}).",
                        other_mod,
                        proj.internal_id,
                        slot + 1,
                        slot
                    ));
                }
            }
        }
        conflicts
    }

    /// Auto-resuelve conflictos moviendo mods activos a casillas vacías de la grilla (0 al 31)
    pub fn auto_resolve_slots(&mut self) {
        let mut used_slots = std::collections::HashSet::new();

        for proj in self.installed_mods.iter_mut() {
            if *self.active_mods.get(&proj.internal_id).unwrap_or(&false) {
                while used_slots.contains(&proj.custom_night.slot_index) {
                    proj.custom_night.slot_index = (proj.custom_night.slot_index + 1) % 32;
                }
                used_slots.insert(proj.custom_night.slot_index);
            }
        }
    }

    /// Elimina un mod del gestor
    pub fn remove_mod(&mut self, id: &str) {
        self.installed_mods.retain(|m| m.internal_id != id);
        self.active_mods.remove(id);
        let _ = fs::remove_file(self.mods_dir.join(format!("{}.fbmod", id)));
        let _ = fs::remove_file(self.mods_dir.join(format!("{}.json", id)));
        self.save_manifest();
    }
}
