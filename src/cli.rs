use clap::Parser;
use std::env;
use std::fs;
use std::io::{Write, Read};
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;


#[derive(Parser, Debug)]
#[command(author, version, about = "CLI para publicar en LinkedIn")]
pub struct Cli {
    /// Texto a publicar directamente desde consola
    pub text: Vec<String>,

    /// Ruta a un archivo de texto con el contenido del post
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Rutas a imágenes a adjuntar (-i foto1.jpg -i foto2.jpg)
    #[arg(short = 'i', long = "image", value_name = "IMAGE_PATH", num_args = 1..)]
    pub images: Vec<PathBuf>,

    /// Ruta a un documento PDF a adjuntar
    #[arg(short = 'p', long = "pdf", value_name = "PDF_PATH")]
    pub pdf: Option<PathBuf>,

    /// Modo debug: muestra la estructura del post sin enviarlo
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,
}

impl Cli {
    pub fn get_content(&self) -> Result<String, String> {
        if let Some(file_path) = &self.file {
            fs::read_to_string(file_path)
                .map_err(|e| format!("Error leyendo el archivo {:?}: {}", file_path, e))
        } else if !self.text.is_empty() {
            Ok(self.text.join(" "))
        } else {
	    self.capture_from_editor()
        }
    }

    fn capture_from_editor(&self) -> Result<String, String> {
        // Determinamos el editor prefiriendo $VISUAL o $EDITOR, o fallback a nano/vi
        let editor = env::var("VISUAL")
            .or_else(|_| env::var("EDITOR"))
            .unwrap_or_else(|_| "nano".to_string());

        // Creamos un archivo temporal
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| format!("Error al crear archivo temporal: {}", e))?;

        // Opcional: Escribir instrucciones o una plantilla inicial dentro del archivo
        let template = "\n# Escribe el contenido de tu post en LinkedIn arriba.\n# Los cambios guardados se usarán como texto final.";
        temp_file.write_all(template.as_bytes())
            .map_err(|e| format!("Error al escribir plantilla en el archivo temporal: {}", e))?;

        let temp_path = temp_file.path().to_path_buf();

        // Lanzamos el proceso del editor
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .map_err(|e| format!("Error al intentar ejecutar el editor '{}': {}", editor, e))?;

        if !status.success() {
            return Err(format!("El editor '{}' terminó con un código de error", editor));
        }

        // Leemos el contenido redactado por el usuario
        let mut content = String::new();
        let mut file = fs::File::open(&temp_path)
            .map_err(|e| format!("Error al reabrir el archivo temporal: {}", e))?;
        
        file.read_to_string(&mut content)
            .map_err(|e| format!("Error al leer el archivo temporal: {}", e))?;

        // Limpiamos las líneas de comentario si decidiste incluir la plantilla opcional
        let filtered_content: String = content
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        if filtered_content.is_empty() {
            Err("Publicación cancelada: el texto está vacío.".to_string())
        } else {
            Ok(filtered_content)
        }
    }
}
