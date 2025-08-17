pub mod session;

use crate::core::FileMetadata;
use anyhow::Result;
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
        println!("[DEBUG] Obteniendo URL de descarga para el archivo: {}", file_id);

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
            .await?
            .json::<serde_json::Value>()
            .await?;

        if let Some(download_data) = response.get("g") {
            if let Some(download_url) = download_data.as_str() {
                println!("[DEBUG] URL de descarga: {}", download_url);
                Ok(download_url.to_string())
            } else {
                Err(anyhow::anyhow!("No se pudo obtener la URL de descarga"))
            }
        } else {
            Err(anyhow::anyhow!("No se pudo obtener la URL de descarga"))
        }
    }

    /// Obtiene información de un archivo
    pub async fn get_file_info(&self, file_id: &str) -> Result<FileMetadata> {
        println!("[DEBUG] Obteniendo información del archivo: {}", file_id);

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
            .await?
            .json::<Vec<MegaFileResponse>>()
            .await?;

        if let Some(file_response) = response.get(0) {
            // The file attributes are encrypted and base64 encoded.
            // let key = crate::crypto::url_base64_to_bin(&self.session.master_key.as_ref().unwrap().iter().map(|&c| c as u8).collect::<Vec<u8>>().iter().map(|&c| c as char).collect::<String>())?;
            // let decrypted_attributes =
            //     crate::crypto::decrypt_file_attributes(&file_response.at, &key)?;
            // let attributes: MegaFileAttribute = serde_json::from_str(&decrypted_attributes)?;

            let file_metadata = FileMetadata {
                name: "decrypted_file_name".to_string(), //attributes.n,
                size: file_response.s,
                key: "placeholder_key".to_string(),
            };

            println!(
                "[DEBUG] Información del archivo: {}, tamaño: {} bytes",
                file_metadata.name, file_metadata.size
            );

            Ok(file_metadata)
        } else {
            Err(anyhow::anyhow!("No se pudo obtener la información del archivo"))
        }
    }
}