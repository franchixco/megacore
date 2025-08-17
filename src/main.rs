
use clap::{Parser, Subcommand};
use megacore::core::{Download, DownloadStatus};
use megacore::downloader::manager::DownloadManager;
use megacore::mega_api::session::Session;
use megacore::mega_api::MegaApiClient;
use std::path::PathBuf;
use log::{info, error};

#[derive(Parser)]
#[command(name = "megacore", version, about = "MEGA helper process (Rust core)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Descargar un enlace MEGA
    Get {
        /// URL de MEGA a descargar
        link: String,

        /// Ruta donde guardar el archivo descargado
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Número de conexiones paralelas
        #[arg(short, long, default_value_t = 6)]
        slots: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Get { link, output, slots } => {
            info!("[megacore] Descargando: {}", link);
            info!("[megacore] Ruta de salida: {}", output.display());
            info!("[megacore] Conexiones: {}", slots);

            let mut download_manager = DownloadManager::new();

            let download = Download {
                url: link,
                download_path: output.to_str().unwrap_or(".").to_string(),
                file_metadata: None,
                progress: 0,
                status: DownloadStatus::Pending,
            };

            download_manager.add_download(download);

            while download_manager.has_downloads() {
                if let Some(download) = download_manager.get_next_download() {
                    // Inicializar una sesión de MEGA
                    info!("[megacore] Inicializando sesión de MEGA...");
                    let mut session = Session::default();
                    if let Err(e) = session.init().await {
                        error!("Error al inicializar la sesión de MEGA: {}", e);
                        continue;
                    }

                    // Usar la implementación real del downloader con la sesión de MEGA
                    let api_client = MegaApiClient::new(session);
                    let mut downloader =
                        megacore::downloader::Downloader::new(download)
                            .with_slots(slots)
                            .with_api_client(api_client);

                    // Iniciar la descarga
                    info!("[megacore] Iniciando descarga...");
                    if let Err(e) = downloader.download().await {
                        error!("Error al descargar el archivo: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
