mod vms;

#[macro_use]
extern crate lazy_static;
extern crate reqwest;

use serde::Deserialize;
use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Error};

use vms::*;

#[get("/start")]
async fn start_containers(data: web::Data<AppData>) -> impl Responder {
    let mut pool = data.runner.clone();
    
    return match pool.start_all() {
        Ok(_) => Ok(HttpResponse::Ok()),
        Err(s) => Err(Error::from(HttpResponse::from(s)))
    }
}

#[get("/shutdown")]
async fn shutdown_containers(data: web::Data<AppData>) -> impl Responder {
    let mut pool = data.runner.clone();
    
    return match pool.shutdown_all() {
        Ok(_) => Ok(HttpResponse::Ok()),
        Err(s) => Err(Error::from(HttpResponse::from(s)))
    }
}

#[derive(Deserialize, Debug, Clone)]
struct CodeData{
    preparation: String,
    code: String,
    execution: String,
}

#[post("/execute")]
async fn execute(data: web::Data<AppData>, post_data: web::Json<CodeData>) -> impl Responder {
    let mut pool = data.runner.clone();

    let lang = "python".to_string();
    let prep = post_data.preparation.clone();
    let code = post_data.code.clone();
    let exec = post_data.execution.clone();
    
    return match pool.prepare_and_execute_code(lang.clone(), prep, exec, code, true) {
        Ok(a) => Ok(a),
        Err(s) => Err(Error::from(HttpResponse::from(s)))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut data = AppData::new(3);
    data.runner.start_all().expect("Error while starting containers");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive().supports_credentials())
            .app_data(web::Data::new(data.clone()))
            .service(start_containers)
            .service(shutdown_containers)
            .service(execute)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}