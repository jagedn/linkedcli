use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenData {
    pub access_token: String,
    pub author_urn: String,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}


static TOKEN_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn token_path() -> &'static PathBuf {
    TOKEN_PATH.get_or_init(|| {
        let home = dirs::home_dir().expect("No se pudo obtener el HOME del usuario");
        let dir = home.join(".linkedincli");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(".linkedin_token.json")
    })
}


pub async fn get_or_engine_auth() -> Result<TokenData, Box<dyn std::error::Error>> {
    if Path::new(token_path()).exists() {
        let content = fs::read_to_string(token_path())?;
        if let Ok(data) = serde_json::from_str::<TokenData>(&content) {
            return Ok(data);
        }
    }

    let client_id = env::var("LINKEDIN_CLIENT_ID")?;
    let client_secret = env::var("LINKEDIN_CLIENT_SECRET")?;
    let redirect_uri = env::var("LINKEDIN_REDIRECT_URI")?;

    let auth_url = format!(
        "https://www.linkedin.com/oauth/v2/authorization?response_type=code&client_id={}&redirect_uri={}&scope=openid%20profile%20w_member_social",
        client_id,
        urlencoding::encode(&redirect_uri)
    );

    println!("Abriendo el navegador para autorizar la aplicación...");
    webbrowser::open(&auth_url)?;

    let code = listen_for_code()?;
    let client = reqwest::Client::new();

    let token_res: TokenResponse = client
        .post("https://www.linkedin.com/oauth/v2/accessToken")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await?
        .json()
        .await?;

    let user_info: UserInfo = client
        .get("https://api.linkedin.com/v2/userinfo")
        .bearer_auth(&token_res.access_token)
        .send()
        .await?
        .json()
        .await?;

    let token_data = TokenData {
        access_token: token_res.access_token,
        author_urn: format!("urn:li:person:{}", user_info.sub),
    };

    fs::write(token_path(), serde_json::to_string_pretty(&token_data)?)?;
    Ok(token_data)
}

fn listen_for_code() -> Result<String, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let (mut stream, _) = listener.accept()?;
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap()?;

    let code = request_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| path.split("code=").nth(1))
        .and_then(|params| params.split('&').next())
        .ok_or("No se encontró el parámetro 'code' en la redirección.")?
        .to_string();

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nAutenticacion completada. Puedes cerrar esta ventana.";
    stream.write_all(response.as_bytes())?;

    Ok(code)
}
