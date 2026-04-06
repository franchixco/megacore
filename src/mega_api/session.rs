
use anyhow::Result;
use rand::rngs::OsRng;
use rand::Rng;

#[derive(Debug, Clone, Default)]
pub struct Session {
    pub seq_no: u64,
    sid: Option<String>,
    master_key: Option<Vec<u32>>,
    rsa_priv_key: Option<Vec<Vec<u8>>>,
    password_aes: Option<Vec<u32>>,
    user_hash: Option<String>,
    root_id: Option<String>,
    inbox_id: Option<String>,
    email: Option<String>,
    full_email: Option<String>,
    trashbin_id: Option<String>,
    req_id: String,
    account_version: i32,
    salt: Option<String>,
}

impl Session {
    pub fn new() -> Self {
        let mut rng = OsRng;
        let seq_no = rng.gen::<u64>() & 0xffffffff;
        let req_id = Self::gen_id(10);

        Self {
            seq_no,
            sid: None,
            master_key: None,
            rsa_priv_key: None,
            password_aes: None,
            user_hash: None,
            root_id: None,
            inbox_id: None,
            email: None,
            full_email: None,
            trashbin_id: None,
            req_id,
            account_version: -1,
            salt: None,
        }
    }

    fn gen_id(length: usize) -> String {
        let mut rng = OsRng;
        let chars: Vec<char> =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
                .chars()
                .collect();
        (0..length)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect()
    }

    /// Inicializa la sesión
    pub async fn init(&mut self) -> Result<()> {
        println!("[INFO] Inicializando sesión de MEGA...");

        // Generar un nuevo ID de solicitud
        self.req_id = Self::gen_id(10);

        // En una implementación real, aquí se realizaría una petición
        // a la API de MEGA para obtener información básica de la sesión
        // como la versión de la API, etc.

        // Por ahora, simplemente simulamos una inicialización exitosa
        println!("[INFO] Sesión inicializada correctamente");

        Ok(())
    }

    pub async fn login(&mut self, email: &str, _password: &str) -> Result<()> {
        // Implementación básica del login
        self.email = Some(email.to_string());

        // Aquí iría la lógica de autenticación con MEGA
        // 1. Generar hash de la contraseña
        // 2. Realizar solicitud de login
        // 3. Procesar respuesta y extraer claves

        Ok(())
    }

    pub async fn fetch_nodes(&mut self) -> Result<()> {
        // Implementación básica para obtener los nodos del usuario
        // Aquí se obtendrían los IDs de carpetas importantes como root, inbox, etc.

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_id_length() {
        let length = 10;
        let id = Session::gen_id(length);
        assert_eq!(id.len(), length);
    }

    #[test]
    fn test_gen_id_charset() {
        let length = 100;
        let id = Session::gen_id(length);
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        for c in id.chars() {
            assert!(chars.contains(c));
        }
    }

    #[test]
    fn test_session_new_initializes_req_id() {
        let session = Session::new();
        assert_eq!(session.req_id.len(), 10);
    }
}
