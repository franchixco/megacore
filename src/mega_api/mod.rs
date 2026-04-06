pub mod session;

use crate::core::FileMetadata;
use crate::crypto;
use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

pub const API_URL: &str = "https://g.api.mega.co.nz";

#[derive(Deserialize, Debug)]
struct MegaFileAttribute {
    n: String,
}

#[derive(Deserialize, Debug)]
struct MegaFileResponse {
    s: u64,
    at: String,
    k: Option<String>,
}

pub struct MegaApiClient {
    client: Client,
    session: session::Session,
}

impl MegaApiClient {
    pub fn new(session: session::Session) -> Self {
        Self {
            client: Client::new(),
            session,
        }
    }

    /// Obtiene la URL de descarga para un archivo
    pub async fn get_download_url(&self, file_id: &str) -> Result<String> {
        info!("Obteniendo URL de descarga para el archivo: {}", file_id);

        let request = json!({
            "a": "g",
            "g": 1,
            "p": file_id,
            "ssl": 2
        });

        let url = format!("{}/cs?id={}", API_URL, self.session.seq_no);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Error al obtener la URL de descarga: {}", response.status());
            return Err(anyhow::anyhow!(
                "Error al obtener la URL de descarga: {}",
                response.status()
            ));
        }

        let response_json = response.json::<serde_json::Value>().await?;

        if let Some(download_data) = response_json.get("g") {
            if let Some(download_url) = download_data.as_str() {
                info!("URL de descarga obtenida: {}", download_url);
                Ok(download_url.to_string())
            } else {
                warn!("No se pudo obtener la URL de descarga del JSON de respuesta");
                Err(anyhow::anyhow!("No se pudo obtener la URL de descarga"))
            }
        } else {
            warn!("No se pudo obtener la URL de descarga del JSON de respuesta");
            Err(anyhow::anyhow!("No se pudo obtener la URL de descarga"))
        }
    }

    /// Obtiene información de un archivo
    pub async fn get_file_info(&self, file_id: &str, file_key: &str) -> Result<FileMetadata> {
        info!("Obteniendo información del archivo: {}", file_id);

        let request = json!({
            "a": "g",
            "p": file_id,
            "ssl": 2
        });

        let url = format!("{}/cs?id={}", API_URL, self.session.seq_no);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Error al obtener la información del archivo: {}", response.status());
            return Err(anyhow::anyhow!(
                "Error al obtener la información del archivo: {}",
                response.status()
            ));
        }

        let response_json = response.json::<Vec<MegaFileResponse>>().await?;

        if let Some(file_response) = response_json.get(0) {
            let decoded_file_key = crypto::url_base64_to_bin(file_key)?;

            let decrypted_attributes = crypto::decrypt_file_attributes(&file_response.at, &decoded_file_key)?;
            let attributes: MegaFileAttribute = serde_json::from_str(&decrypted_attributes)?;

            let file_metadata = FileMetadata {
                name: attributes.n,
                size: file_response.s,
                key: file_response.k.clone().unwrap_or_default(),
            };

            info!(
                "Información del archivo obtenida: {}, tamaño: {} bytes",
                file_metadata.name,
                file_metadata.size
            );

            Ok(file_metadata)
        } else {
            warn!("No se pudo obtener la información del archivo del JSON de respuesta");
            Err(anyhow::anyhow!("No se pudo obtener la información del archivo"))
        }
    }
}
