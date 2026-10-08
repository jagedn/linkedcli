use crate::auth::TokenData;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct InitializeUploadResponse {
    value: InitializeUploadValue,
}

#[derive(Deserialize)]
struct InitializeUploadValue {
    #[serde(alias = "image", alias = "document")]
    media_urn: String,
    #[serde(rename = "uploadUrl")]
    upload_url: String,
}

pub struct LinkedInClient {
    http: Client,
    token_data: TokenData,
    version: String,
}

impl LinkedInClient {
    pub fn new(token_data: TokenData) -> Self {
        let version = env::var("LINKEDIN_VERSION").unwrap_or_else(|_| "202601".to_string());
        Self {
            http: Client::new(),
            token_data,
            version,
        }
    }

    /// Subida de imágenes
    pub async fn upload_image(&self, image_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        if !image_path.exists() {
            return Err(format!("El archivo de imagen no existe: {:?}", image_path).into());
        }

        let init_payload = json!({
            "initializeUploadRequest": {
                "owner": self.token_data.author_urn
            }
        });

        let init_res = self
            .http
            .post("https://api.linkedin.com/rest/images?action=initializeUpload")
            .bearer_auth(&self.token_data.access_token)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", &self.version)
            .json(&init_payload)
            .send()
            .await?;

        if !init_res.status().is_success() {
            let err_body = init_res.text().await?;
            return Err(format!("Error inicializando la subida de imagen: {}", err_body).into());
        }

        let init_data: InitializeUploadResponse = init_res.json().await?;

        let image_bytes = fs::read(image_path)?;
        let upload_res = self
            .http
            .put(&init_data.value.upload_url)
            .bearer_auth(&self.token_data.access_token)
            .body(image_bytes)
            .send()
            .await?;

        if !upload_res.status().is_success() {
            let err_body = upload_res.text().await?;
            return Err(format!("Error al subir el binario de la imagen: {}", err_body).into());
        }

        Ok(init_data.value.media_urn)
    }

    /// Subida de documentos (PDFs)
    pub async fn upload_document(&self, doc_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        if !doc_path.exists() {
            return Err(format!("El archivo PDF no existe: {:?}", doc_path).into());
        }

        let init_payload = json!({
            "initializeUploadRequest": {
                "owner": self.token_data.author_urn
            }
        });

        let init_res = self
            .http
            .post("https://api.linkedin.com/rest/documents?action=initializeUpload")
            .bearer_auth(&self.token_data.access_token)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", &self.version)
            .json(&init_payload)
            .send()
            .await?;

        if !init_res.status().is_success() {
            let err_body = init_res.text().await?;
            return Err(format!("Error inicializando la subida de documento: {}", err_body).into());
        }

        let init_data: InitializeUploadResponse = init_res.json().await?;

        let doc_bytes = fs::read(doc_path)?;
        let upload_res = self
            .http
            .put(&init_data.value.upload_url)
            .bearer_auth(&self.token_data.access_token)
            .header("Content-Type", "application/pdf")
            .body(doc_bytes)
            .send()
            .await?;

        if !upload_res.status().is_success() {
            let err_body = upload_res.text().await?;
            return Err(format!("Error al subir el binario del PDF: {}", err_body).into());
        }

        Ok(init_data.value.media_urn)
    }

    /// Publicar texto con (opcional) imágenes o documento
    pub async fn publish_post(
        &self,
        text: &str,
        image_urns: &[String],
        document_info: Option<(&str, &str)>, // (URN, Title)
        is_debug: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = json!({
            "author": self.token_data.author_urn,
            "commentary": text,
            "visibility": "PUBLIC",
            "distribution": {
                "feedDistribution": "MAIN_FEED",
                "targetEntities": [],
                "thirdPartyDistributionChannels": []
            },
            "lifecycleState": "PUBLISHED",
            "isReshareDisabledByAuthor": false
        });

        if let Some((doc_urn, title)) = document_info {
            payload["content"] = json!({
                "media": {
                    "id": doc_urn,
                    "title": title
                }
            });
        } else if image_urns.len() == 1 {
            payload["content"] = json!({
                "media": {
                    "id": image_urns[0]
                }
            });
        } else if image_urns.len() > 1 {
            let images_json: Vec<_> = image_urns
                .iter()
                .map(|urn| json!({ "id": urn }))
                .collect();

            payload["content"] = json!({
                "multiImage": {
                    "images": images_json
                }
            });
        }

        if is_debug {
            println!("[DEBUG MODE] Payload JSON que se enviaría a LinkedIn:\n");
            println!("{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        let res = self
            .http
            .post("https://api.linkedin.com/rest/posts")
            .bearer_auth(&self.token_data.access_token)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("LinkedIn-Version", &self.version)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await?;
            return Err(format!("Error API LinkedIn ({}): {}", status, err_body).into());
        }

        println!("Post publicado con éxito.");
        Ok(())
    }
}
