mod auth;
mod cli;
mod linkedin;

use clap::Parser;
use cli::Cli;
use linkedin::LinkedInClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Validar conflicto de adjuntos
    if !cli.images.is_empty() && cli.pdf.is_some() {
        eprintln!("Error: No se pueden adjuntar imágenes y un PDF en la misma publicación.");
        std::process::exit(1);
    }

    let post_text = match cli.get_content() {
        Ok(text) => text,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };

    if post_text.trim().is_empty() {
        eprintln!("Error: El contenido a publicar está vacío.");
        std::process::exit(1);
    }

    let token_data = auth::get_or_engine_auth().await?;
    let client = LinkedInClient::new(token_data);

    let mut image_urns = Vec::new();
    let mut document_info = None;

    if let Some(pdf_path) = &cli.pdf {
        println!("Subiendo documento PDF {:?}...", pdf_path);
        let urn = client.upload_document(pdf_path).await?;
        let doc_name = pdf_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Documento");
        document_info = Some((urn, doc_name.to_string()));
    } else {
        for image_path in &cli.images {
            println!("Subiendo imagen {:?}...", image_path);
            let urn = client.upload_image(image_path).await?;
            image_urns.push(urn);
        }
    }

    let doc_ref = document_info
        .as_ref()
        .map(|(urn, title)| (urn.as_str(), title.as_str()));

    client
        .publish_post(&post_text, &image_urns, doc_ref, cli.debug)
        .await?;

    Ok(())
}
