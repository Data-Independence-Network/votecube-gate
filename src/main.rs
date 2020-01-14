use std::fs::File;
use std::io::BufReader;

use actix_web::{App, client, Error, HttpRequest, HttpResponse, HttpServer, web};
use rustls::{NoClientAuth, ServerConfig};
use rustls::internal::pemfile::{certs, rsa_private_keys};

/// simple handle
async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<client::Client>,
) -> Result<HttpResponse, Error> {
//    let mut url = req.uri().path().clone();
    let mut query_string = String::new();
    match req.uri().query() {
        Some(query) => {
            query_string = format!("?{}", query);
        }
        None => {
            // Nothing to do
        }
    }
    let path = req.uri().path();
    let url;
    if req.uri().path().starts_with("/get")
        || req.uri().path().starts_with("/list") {
        url = format!("http://localhost:8444{}{}", path, query_string);
    } else if req.uri().path().starts_with("/put") {
        url = format!("http://localhost:8445{}{}", path, query_string);
    } else if req.uri().path().starts_with("/ui") {
//        url = format!("http://localhost:3000{}{}", &path[3..], query_string);
        url = format!("http://localhost:3000{}{}", path, query_string);
    } else {
        return Ok(HttpResponse::BadRequest()
            .header("Cache-Control", "public, max-age=86400")
            .finish());
    }

//    println!("Forward URL: {}", url);

    let forwarded_req = client
        .request_from(url, req.head())
        .no_decompress();

    let res = forwarded_req.send_body(body).await.map_err(Error::from)?;

//    println!("Response Status: {}", res.status());

    let mut client_resp = HttpResponse::build(res.status());
    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in
        res.headers().iter().filter(|(h, _)| *h != "connection")
        {
            client_resp.header(header_name.clone(), header_value.clone());
        }

    Ok(client_resp.streaming(res))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }
    env_logger::init();

    // load ssl keys
    let mut config = ServerConfig::new(NoClientAuth::new());
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    let cert_chain = certs(cert_file).unwrap();
    let mut keys = rsa_private_keys(key_file).unwrap();
    config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

    HttpServer::new(move || {
        App::new()
            .data(client::Client::new())
            // enable logger
//            .wrap(middleware::Logger::default())
            // handle paths
            .default_service(web::route().to(forward))
    })
        .bind_rustls("127.0.0.1:8443", config)?
        .start()
        .await
}
