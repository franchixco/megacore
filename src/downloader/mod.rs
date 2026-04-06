pub mod manager;

use crate::core::{Chunk, Download, DownloadStatus, FileMetadata};
use crate::crypto;
use crate::http;
use crate::mega_api::session::Session;
use crate::mega_api::MegaApiClient;
use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::Client;
use std::fs::File;
use std::io::{self, Seek, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task;

pub const CHUNK_SIZE_MULTI: usize = 20;
pub const WORKERS_DEFAULT: usize = 6;

struct ChunkDownloader {
    client: Client,
    url: String,
    chunk: Chunk,
}

impl ChunkDownloader {
    fn new(client: Client, url: String, chunk: Chunk) -> Self {
        Self {
            client,
            url,
            chunk,
        }
    }

    async fn download(mut self) -> Result<Chunk> {
        // Crear la solicitud HTTP con el rango específico
        let range = format!("bytes={}-{}", self.chunk.start, self.chunk.end);

        // Número máximo de intentos
        let max_retries = 3;
        let mut attempt = 0;
        let mut last_error = None;

        // Intentar la descarga con reintentos
        while attempt < max_retries {
            attempt += 1;

            debug!(
                "Descargando chunk {} (intento {}/{}):",
                self.chunk.id, attempt, max_retries
            );

            // Realizar la petición HTTP
            match self.client.get(&self.url).header("Range", &range).send().await {
                Ok(response) => {
                    // Verificar que la respuesta sea correcta
                    if response.status().is_success() {
                        // Verificar que el servidor soporta rangos
                        if let Some(content_range) = response.headers().get("Content-Range") {
                            debug!("Content-Range: {:?}", content_range);
                        } else {
                            warn!(
                                "El servidor no devolvió Content-Range, puede que no soporte rangos"
                            );
                        }

                        // Obtener los bytes
                        match response.bytes().await {
                            Ok(bytes) => {
                                self.chunk.data = Some(bytes.to_vec());
                                return Ok(self.chunk);
                            }
                            Err(e) => {
                                last_error = Some(anyhow::anyhow!(
                                    "Error al leer bytes del chunk {}: {}",
                                    self.chunk.id,
                                    e
                                ));
                                error!("Error al leer bytes: {}", e);
                            }
                        }
                    } else {
                        last_error = Some(anyhow::anyhow!(
                            "Error HTTP al descargar chunk {}: {}",
                            self.chunk.id,
                            response.status()
                        ));
                        error!("Error HTTP: {}", response.status());
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!(
                        "Error de conexión al descargar chunk {}: {}",
                        self.chunk.id,
                        e
                    ));
                    error!("Error de conexión: {}", e);
                }
            }

            // Esperar antes de reintentar
            if attempt < max_retries {
                info!(
                    "Reintentando descarga del chunk {} en 1 segundo...",
                    self.chunk.id
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        // Si llegamos aquí, todos los intentos fallaron
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Error desconocido al descargar chunk {}", self.chunk.id)
        }))
    }
}

struct FileAssembler {
    download: Download,
    temp_file_path: String,
    final_file_path: String,
}

impl FileAssembler {
    fn new(download: Download, temp_file_path: String, final_file_path: String) -> Self {
        Self {
            download,
            temp_file_path,
            final_file_path,
        }
    }

    fn assemble_and_decrypt(&self) -> Result<()> {
        // Mover el archivo temporal a su ubicación final
        std::fs::rename(&self.temp_file_path, &self.final_file_path)?;

        // Verificar la integridad del archivo
        if let Some(file_metadata) = &self.download.file_metadata {
            if !file_metadata.key.is_empty() {
                info!("Verificando integridad del archivo...");
                match crypto::verify_file_integrity(&self.final_file_path, &file_metadata.key) {
                    Ok(true) => {
                        info!("La integridad del archivo ha sido verificada correctamente.");
                    }
                    Ok(false) => {
                        error!("La verificación de la integridad del archivo ha fallado.");
                        return Err(anyhow::anyhow!(
                            "La verificación de la integridad del archivo ha fallado"
                        ));
                    }
                    Err(e) => {
                        error!("Error al verificar la integridad del archivo: {}", e);
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Estructura principal para manejar descargas
pub struct Downloader {
    download: Download,
    slots: usize,
    client: Client,
    api_client: Option<MegaApiClient>,
}

impl Downloader {
    pub fn new(download: Download) -> Result<Self> {
        let client = http::default_client("MegaBasterd Rust", 30)?;

        Ok(Self {
            download,
            slots: WORKERS_DEFAULT,
            client,
            api_client: None,
        })
    }

    /// Establece el cliente de la API de MEGA
    pub fn with_api_client(mut self, api_client: MegaApiClient) -> Self {
        self.api_client = Some(api_client);
        self
    }

    pub fn with_slots(mut self, slots: usize) -> Self {
        self.slots = slots;
        self
    }

    /// Analiza la URL de MEGA para extraer información
    pub fn parse_url(&mut self) -> Result<()> {
        info!("Analizando URL: {}", self.download.url);
        // Analizar la URL para extraer file_id y file_key
        // Formatos soportados:
        // - https://mega.nz/file/FILE_ID#FILE_KEY
        // - https://mega.nz/#!FILE_ID!FILE_KEY

        let url = self.download.url.as_str();

        // Extraer file_id y file_key
        if url.contains("mega.nz/file/") {
            // Formato nuevo: https://mega.nz/file/FILE_ID#FILE_KEY
            if let Some(file_part) = url.split('/').last() {
                if let Some(hash_pos) = file_part.find('#') {
                    let file_id = &file_part[..hash_pos];
                    let file_key = &file_part[hash_pos + 1..];

                    self.download.file_metadata = Some(FileMetadata {
                        name: format!("downloaded_file_{}", file_id),
                        size: 0, // Se obtendrá después
                        key: file_key.to_string(),
                    });
                    info!("URL analizada correctamente.");
                    return Ok(());
                }
            }
        } else if url.contains("mega.nz/#!") {
            // Formato antiguo: https://mega.nz/#!FILE_ID!FILE_KEY
            if let Some(exclamation_parts) = url.split("#!").nth(1) {
                let parts: Vec<&str> = exclamation_parts.split('!').collect();
                if parts.len() >= 2 {
                    let file_id = parts[0];
                    let file_key = parts[1];

                    self.download.file_metadata = Some(FileMetadata {
                        name: format!("downloaded_file_{}", file_id),
                        size: 0, // Se obtendrá después
                        key: file_key.to_string(),
                    });
                    info!("URL analizada correctamente.");
                    return Ok(());
                }
            }
        }
        warn!("URL de MEGA no válida o no soportada: {}", self.download.url);
        Err(anyhow::anyhow!("URL de MEGA no válida o no soportada"))
    }

    /// Obtiene la información del archivo (tamaño, nombre, etc.)
    pub async fn get_file_info(&mut self) -> Result<()> {
        info!("Obteniendo información del archivo...");
        // Verificar que tenemos un cliente de API o crear uno anónimo
        if self.api_client.is_none() {
            info!("No se ha proporcionado un cliente de API, creando uno anónimo...");
            let session = Session::new();
            self.api_client = Some(MegaApiClient::new(session));
        }

        // Extraer el file_id de la URL si no lo hemos hecho ya
        if self.download.file_metadata.is_none() {
            self.parse_url()?;
        }

        if let Some(file_metadata) = self.download.file_metadata.as_mut() {
            // Extraer el file_id del nombre temporal
            let file_id = file_metadata
                .name
                .strip_prefix("downloaded_file_")
                .ok_or_else(|| anyhow::anyhow!("No se pudo extraer el file_id"))?;

            // Obtener información del archivo usando el cliente de la API de MEGA
            if let Some(api_client) = &self.api_client {
                info!(
                    "Obteniendo información del archivo desde la API de MEGA: {}",
                    file_id
                );

                // En una implementación real, obtendríamos la información del archivo
                // a través de la API de MEGA
                let api_file_info = api_client.get_file_info(file_id, &file_metadata.key).await?;

                // Actualizar la información del archivo con los datos obtenidos
                file_metadata.name = api_file_info.name;
                file_metadata.size = api_file_info.size;

                info!(
                    "Información obtenida: {}, tamaño: {} bytes",
                    file_metadata.name,
                    file_metadata.size
                );
            } else {
                // Si no hay cliente de API disponible, usar valores por defecto
                warn!("No hay cliente de API disponible, usando valores por defecto");
                file_metadata.size = 1024 * 1024 * 10; // 10 MB
            }
        } else {
            return Err(anyhow::anyhow!("No se pudo obtener la metadata del archivo"));
        }

        Ok(())
    }

    /// Descarga el archivo
    pub async fn download(&mut self) -> Result<()> {
        info!("Iniciando descarga para: {}", self.download.url);
        self.download.status = DownloadStatus::Downloading;

        // Verificar que tenemos toda la información necesaria
        if self.download.file_metadata.is_none() {
            if let Err(e) = self.parse_url() {
                error!("Error al analizar la URL: {}", e);
                self.download.status = DownloadStatus::Failed(e.to_string());
                return Err(e);
            }
            if let Err(e) = self.get_file_info().await {
                error!("Error al obtener la información del archivo: {}", e);
                self.download.status = DownloadStatus::Failed(e.to_string());
                return Err(e);
            }
        }

        let file_metadata = self.download.file_metadata.as_ref().unwrap();
        let file_size = file_metadata.size;
        let temp_file_path_str = Path::new(&self.download.download_path)
            .join(&format!("{}.mctemp", file_metadata.name))
            .to_string_lossy()
            .to_string();
        let final_file_path_str = Path::new(&self.download.download_path)
            .join(&file_metadata.name)
            .to_string_lossy()
            .to_string();

        info!("Descargando archivo: {}", file_metadata.name);
        info!("Tamaño: {} bytes", file_size);
        info!("Guardando en: {}", final_file_path_str);

        // Obtener la URL de descarga real de la API de MEGA
        let download_url = if let Some(api_client) = &self.api_client {
            // Extraer el file_id del nombre del archivo temporal
            // En una implementación real, esto vendría de los metadatos del archivo
            let file_id = if let Some(id) = file_metadata.name.strip_prefix("downloaded_file_") {
                id
            } else {
                // Si no podemos extraer el ID del nombre, usamos el ID que obtuvimos al parsear la URL
                let url = self.download.url.as_str();
                if url.contains("mega.nz/file/") {
                    if let Some(file_part) = url.split('/').last() {
                        if let Some(hash_pos) = file_part.find('#') {
                            &file_part[..hash_pos]
                        } else {
                            ""
                        }
                    } else {
                        ""
                    }
                } else if url.contains("mega.nz/#!") {
                    if let Some(exclamation_parts) = url.split("#!").nth(1) {
                        let parts: Vec<&str> = exclamation_parts.split('!').collect();
                        if parts.len() >= 2 {
                            parts[0]
                        } else {
                            ""
                        }
                    } else {
                        ""
                    }
                } else {
                    ""
                }
            };

            // Obtener la URL de descarga real
            match api_client.get_download_url(file_id).await {
                Ok(url) => {
                    info!("URL de descarga obtenida de la API: {}", url);
                    url
                }
                Err(e) => {
                    error!("Error al obtener la URL de descarga: {}", e);
                    self.download.status = DownloadStatus::Failed(e.to_string());
                    return Err(e);
                }
            }
        } else {
            // Si no hay cliente de API, usamos la URL original (esto no funcionaría en una implementación real)
            warn!("No hay cliente de API activo, usando URL original (esto probablemente fallará)");
            self.download.url.clone()
        };

        info!("URL de descarga: {}", download_url);

        // Crear el archivo temporal
        let temp_file = Arc::new(Mutex::new(File::create(&temp_file_path_str)?));

        // Calcular el tamaño de cada chunk
        let chunk_size = (file_size / self.slots as u64).max(1024 * 1024); // Mínimo 1 MB por chunk

        // Crear los chunks
        let mut chunks = Vec::new();
        for i in 0..self.slots {
            let start = i as u64 * chunk_size;
            let end = if i == self.slots - 1 {
                file_size - 1
            } else {
                start + chunk_size - 1
            };

            chunks.push(Chunk {
                id: i as u64,
                start,
                end,
                data: None,
            });
        }

        info!(
            "Iniciando descarga con {} conexiones paralelas",
            self.slots
        );

        // Crear canales para comunicación entre workers
        let (tx, mut rx) = mpsc::channel(self.slots);

        // Lanzar workers para descargar chunks
        for chunk in chunks {
            let client = self.client.clone();
            let url = download_url.clone();
            let tx = tx.clone();

            task::spawn(async move {
                let chunk_downloader = ChunkDownloader::new(client, url, chunk);
                match chunk_downloader.download().await {
                    Ok(chunk) => {
                        if let Err(e) = tx.send(chunk).await {
                            error!("Error al enviar el chunk: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error descargando chunk: {}", e);
                    }
                }
            });
        }

        // Cerrar el transmisor original
        drop(tx);

        // Recibir y escribir chunks
        let mut received_chunks = 0;
        let total_chunks = self.slots;
        let mut total_bytes_downloaded: u64 = 0;
        let mut last_progress_time = std::time::Instant::now();
        let start_time = std::time::Instant::now();

        info!("Iniciando recepción de datos...");

        while let Some(chunk) = rx.recv().await {
            if let Some(data) = chunk.data {
                let chunk_size = data.len() as u64;
                total_bytes_downloaded += chunk_size;

                // Escribir el chunk en el archivo de forma no bloqueante
                let temp_file_clone = Arc::clone(&temp_file);
                let start_pos = chunk.start;

                task::spawn_blocking(move || -> Result<()> {
                    let mut file = temp_file_clone.lock().map_err(|_| anyhow::anyhow!("Error al bloquear el archivo"))?;
                    file.seek(io::SeekFrom::Start(start_pos))?;
                    file.write_all(&data)?;
                    Ok(())
                })
                .await
                .map_err(|e| anyhow::anyhow!("Error en la tarea de escritura: {}", e))??;

                // Calcular velocidad de descarga
                let elapsed = last_progress_time.elapsed();
                if elapsed.as_secs() >= 1 || received_chunks == 0 || received_chunks == total_chunks - 1 {
                    let total_elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if total_elapsed > 0.0 {
                        total_bytes_downloaded as f64 / total_elapsed / 1024.0 / 1024.0 // MB/s
                    } else {
                        0.0
                    };

                    let eta = if speed > 0.0 {
                        let remaining_bytes = file_size - total_bytes_downloaded;
                        remaining_bytes as f64 / (speed * 1024.0 * 1024.0)
                    } else {
                        0.0
                    };

                    last_progress_time = std::time::Instant::now();

                    // Formatear el tiempo estimado restante
                    let eta_str = if eta > 0.0 {
                        let eta_secs = eta as u64;
                        let eta_mins = eta_secs / 60;
                        let eta_hours = eta_mins / 60;

                        if eta_hours > 0 {
                            format!("{:02}:{:02}:{:02}", eta_hours, eta_mins % 60, eta_secs % 60)
                        } else {
                            format!("{:02}:{:02}", eta_mins, eta_secs % 60)
                        }
                    } else {
                        "--:--".to_string()
                    };

                    // Mostrar progreso
                    let progress = (received_chunks * 100) / total_chunks;
                    let downloaded_mb = total_bytes_downloaded as f64 / 1024.0 / 1024.0;
                    let total_mb = file_size as f64 / 1024.0 / 1024.0;

                    info!(
                        "Progreso: {}% ({}/{} chunks) | {:.2}/{:.2} MB | {:.2} MB/s | ETA: {}",
                        progress,
                        received_chunks,
                        total_chunks,
                        downloaded_mb,
                        total_mb,
                        speed,
                        eta_str
                    );
                }
            } else {
                warn!("Chunk {} recibido sin datos", chunk.id);
            }

            received_chunks += 1;

            if received_chunks == total_chunks {
                break;
            }
        }

        // Verificar que se hayan recibido todos los chunks
        if received_chunks < total_chunks {
            let error_message = format!(
                "Descarga incompleta: solo se recibieron {}/{} chunks",
                received_chunks,
                total_chunks
            );
            error!("{}", error_message);
            self.download.status = DownloadStatus::Failed(error_message.clone());
            return Err(anyhow::anyhow!(error_message));
        }

        let file_assembler = FileAssembler::new(
            self.download.clone(),
            temp_file_path_str,
            final_file_path_str,
        );

        let assemble_result = task::spawn_blocking(move || {
            file_assembler.assemble_and_decrypt()
        })
        .await;

        match assemble_result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                error!("Error al ensamblar y descifrar el archivo: {}", e);
                self.download.status = DownloadStatus::Failed(e.to_string());
                return Err(e);
            }
            Err(e) => {
                let err_msg = format!("Error en la tarea de ensamblado: {}", e);
                error!("{}", err_msg);
                self.download.status = DownloadStatus::Failed(err_msg.clone());
                return Err(anyhow::anyhow!(err_msg));
            }
        }

        info!("Descarga completada para: {}", self.download.url);
        self.download.status = DownloadStatus::Completed;
        Ok(())
    }
}

/// Función principal para descargar un archivo de MEGA
pub async fn download_file(url: &str, download_path: &str) -> Result<()> {
    let download = Download {
        url: url.to_string(),
        download_path: download_path.to_string(),
        file_metadata: None,
        progress: 0,
        status: DownloadStatus::Pending,
    };
    let session = Session::new();
    let api_client = MegaApiClient::new(session);
    let mut downloader = Downloader::new(download)?.with_api_client(api_client);
    downloader.download().await
}