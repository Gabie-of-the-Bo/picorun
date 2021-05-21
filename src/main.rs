mod vms;

#[macro_use]
extern crate lazy_static;
extern crate reqwest;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, Error};

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

#[get("/execute")]
async fn execute(data: web::Data<AppData>) -> impl Responder {
    let mut pool = data.runner.clone();

    let lang = "python".to_string();
    let exec = "write_output(str(sol(10)))".to_string();
    let code = "import numpy as np
def sol(n):
    return np.ones(n) * 5".to_string();
    
    return match pool.prepare_and_execute_code(lang.clone(), String::new(), exec.clone(), code.clone(), true) {
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
            .app_data(web::Data::new(data.clone()))
            .service(start_containers)
            .service(shutdown_containers)
            .service(execute)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}