use std::process::Command;

use std::iter::FromIterator;
use std::collections::HashSet;

use rand::Rng;

use std::sync::{Arc, Mutex};

lazy_static! {
    static ref RUNNER_IMAGE: String = "picorun-runner".to_string();
}

#[derive(Clone, Debug)]
pub struct LanguageService{
    pub id: Option<String>,
    pub port: Option<u32>,
    paused: bool
}

impl LanguageService{
    pub fn new() -> LanguageService{
        return LanguageService{
            id: None,
            port: None,
            paused: false
        };
    }

    fn exec_command(args: &[&str]) -> Result<std::process::Output, std::io::Error>{
        return if cfg!(target_os = "windows") {
            Command::new("cmd").arg("/C").args(args).output()

        } else {
            Command::new("sh").arg("-c").args(args).output()
        };
    }

    pub fn is_started(&self) -> bool{
        return self.id.is_some() && self.port.is_some();
    }

    pub fn start(&mut self, port: u32) -> Result<(), String>{
        if !self.is_started(){
            return match LanguageService::exec_command(&[
                "docker", "run", "-d", "--rm", "-p", format!("{}:5000/tcp", port).as_str(), RUNNER_IMAGE.as_str(),
                "sh", "-c", "cd home/picorun-runner && RUST_LOG=info ./target/release/server"
            ]) {
                Ok(out) => {
                    self.id = Some(String::from_utf8(out.stdout).unwrap().trim().to_string());
                    self.port = Some(port);
                    Ok(())
                },
                Err(err) => Err(err.to_string())
            }
        } 
        
        return Ok(());
    }

    pub fn shutdown(&mut self) -> Result<(), String>{
        if self.is_started(){
            return match LanguageService::exec_command(&["docker", "rm", "-f", self.id.as_deref().unwrap()]) {
                Ok(_) => {
                    self.id = None;
                    self.port = None;
                    Ok(())
                },
                Err(err) => Err(err.to_string())
            }
        } 
        
        return Ok(());
    }

    pub fn pause(&mut self) -> Result<(), String>{
        if self.is_started() && !self.paused {
            return match LanguageService::exec_command(&["docker", "pause", self.id.as_deref().unwrap()]) {
                Ok(_) => {
                    self.paused = true;
                    Ok(())
                },
                Err(err) => Err(err.to_string())
            }
        } 
        
        return Ok(());
    }

    pub fn unpause(&mut self) -> Result<(), String>{
        if self.is_started() && self.paused {
            return match LanguageService::exec_command(&["docker", "unpause", self.id.as_deref().unwrap()]) {
                Ok(_) => {
                    self.paused = false;
                    Ok(())
                },
                Err(err) => Err(err.to_string())
            }
        } 
        
        return Ok(());
    }

    pub fn write_preparation(&self, language: String, code: String) -> Result<(), String>{
        let url = format!("http://localhost:{}/{}/write-prep", self.port.as_ref().unwrap(), language);

        let client = reqwest::blocking::Client::new();

        return match client.post(url).body(code).send() {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string())
        };
    }

    pub fn write_execution(&self, language: String, code: String) -> Result<(), String>{
        let url = format!("http://localhost:{}/{}/write-exec", self.port.as_ref().unwrap(), language);

        let client = reqwest::blocking::Client::new();

        return match client.post(url).body(code).send() {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string())
        };
    }

    pub fn write_code(&self, language: String, code: String) -> Result<(), String>{
        let url = format!("http://localhost:{}/{}/write-code", self.port.as_ref().unwrap(), language);

        let client = reqwest::blocking::Client::new();

        return match client.post(url).body(code).send() {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string())
        };
    }

    pub fn execute_code(&self, language: String) -> Result<String, String>{
        let url = format!("http://localhost:{}/{}/execute", self.port.as_ref().unwrap(), language);

        let client = reqwest::blocking::Client::new();

        return match client.get(url).send() {
            Ok(output) => Ok(output.text().unwrap()),
            Err(err) => Err(err.to_string())
        };
    }
}

impl Drop for LanguageService{
    fn drop(&mut self){
        self.shutdown().expect("Error while shutting down container");
    }
}

#[derive(Clone, Debug)]
pub struct LanguageServicePool{
    services: Arc<Mutex<Vec<Mutex<LanguageService>>>>,
    ports: Arc<Mutex<HashSet<u32>>>
}

impl LanguageServicePool{
    pub fn new(services: usize) -> LanguageServicePool {
        let mut service_list = vec!();

        for _ in 0..services{
            service_list.push(Mutex::new(LanguageService::new()));
        }

        LanguageServicePool{
            services: Arc::new(Mutex::new(service_list)),
            ports: Arc::new(Mutex::new(HashSet::new()))
        }
    }

    fn generate_port(&mut self) -> u32{
        let mut ports_ref = self.ports.lock().unwrap();

        let mut rng = rand::thread_rng();
        let mut res = 0;

        while res == 0 || (*ports_ref).contains(&res) {
            res = rng.gen_range(5000..6000);
        }

        (*ports_ref).insert(res);

        return res;
    }

    fn release_port(&mut self, port: u32){
        (*self.ports.lock().unwrap()).remove(&port);
    }

    pub fn assign_one(&mut self) -> Option<Mutex<LanguageService>>{
        return self.services.lock().unwrap().pop();
    }

    pub fn start_all(&mut self) -> Result<(), String>{
        let n_of_services = self.services.lock().unwrap().len();
        let ports = (0..n_of_services).map(|_| self.generate_port()).collect::<Vec<_>>();

        return Result::from_iter(self.services.lock().unwrap().iter_mut().zip(ports).map(|(s, p)| s.lock().unwrap().start(p)));
    }

    pub fn shutdown_all(&mut self) -> Result<(), String>{
        let ports = self.services.lock().unwrap().iter().filter(|s| s.lock().unwrap().port.is_some())
                                                 .map(|s| s.lock().unwrap().port.clone().unwrap())
                                                 .collect::<Vec<_>>();

        let res = Result::from_iter(self.services.lock().unwrap().iter_mut().map(|s| s.lock().unwrap().shutdown()));
        
        return match res {
            Ok(()) => {
                ports.iter().for_each(|p| self.release_port(*p));
                return Ok(());
            }

            Err(err) => Err(err)
        }
    }

    pub fn prepare_all_environments(&self, language: String, preparation: String, execution: String) -> Result<(), String>{
        return Result::from_iter(self.services.lock().unwrap().iter().map(|i| {
            return i.lock().unwrap().write_execution(language.clone(), execution.clone())
                    .and(i.lock().unwrap().write_preparation(language.clone(), preparation.clone()))
        }));
    }

    pub fn execute_code(&mut self, language: String, code: String, restart_after: bool) -> Result<String, String>{
        let mut service_m_o = None;
        let out;

        while service_m_o.is_none() {
            service_m_o = self.assign_one();
        }

        let service_m = service_m_o.unwrap();

        {
            let mut service = service_m.lock().unwrap();

            out = service.write_code(language.clone(), code).and(service.execute_code(language));
    
            if restart_after{
                service.shutdown().and(service.start(self.generate_port())).unwrap();
            }
        }

        self.services.lock().unwrap().push(service_m);

        return out;
    }

    pub fn prepare_and_execute_code(&mut self, language: String, preparation: String, execution: String, code: String, restart_after: bool) -> Result<String, String>{
        let mut service_m_o = None;
        let out;

        while service_m_o.is_none() {
            service_m_o = self.assign_one();
        }

        let service_m = service_m_o.unwrap();

        {
            let mut service = service_m.lock().unwrap();
        
            out = service.write_execution(language.clone(), execution).and(
                  service.write_preparation(language.clone(), preparation)).and(
                  service.write_code(language.clone(), code)).and(
                  service.execute_code(language));
    
            if restart_after{
                service.shutdown().and(service.start(self.generate_port())).unwrap();
            }
        }

        self.services.lock().unwrap().push(service_m);

        return out;
    }
}


#[derive(Clone)]
pub struct AppData{
    pub runner: LanguageServicePool
}

impl AppData{
    pub fn new(n_of_services: usize) -> AppData{
        return AppData{
            runner: LanguageServicePool::new(n_of_services)
        };
    }
}