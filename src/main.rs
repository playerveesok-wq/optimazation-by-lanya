use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::Local;
// Импорты для sysinfo 0.32
use sysinfo::{System, ProcessRefreshKind, RefreshKind, CpuRefreshKind, ProcessesToUpdate};
use ureq::json;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    api_key: String,
    bot_token: String,
    webhook_url: String,
}

impl Config {
    fn load() -> Self {
        let path = PathBuf::from("config.json");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config;
                }
            }
        }
        Self {
            api_key: String::new(),
            bot_token: String::new(),
            webhook_url: String::new(),
        }
    }

    fn save(&self) {
        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write("config.json", content).ok();
    }
}

struct OptimaApp {
    config: Config,
    system: System,
    last_update: std::time::Instant,
    status_message: String,
    loading: bool,
}

impl OptimaApp {
    fn new() -> Self {
        // Для sysinfo 0.32 используем RefreshKind::new() вместо nothing()
        let refresh_kind = RefreshKind::new().with_processes(ProcessesToUpdate::all());
        
        Self {
            config: Config::load(),
            system: System::new_with_specifics(refresh_kind),
            last_update: std::time::Instant::now(),
            status_message: String::new(),
            loading: false,
        }
    }

    fn update_system_info(&mut self) {
        // Обновляем CPU
        self.system.refresh_cpu_specifics(CpuRefreshKind::everything());
        
        // Обновляем процессы
        // Аргументы: список процессов, boolean (keep_removed), kind
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::all(), 
            false, 
            ProcessRefreshKind::everything()
        );
    }

    // Исправлено: добавлен &mut self, чтобы можно было менять status_message
    fn send_webhook(&mut self, title: &str, description: &str) {
        let payload = json!({
            "embeds": [{
                "title": title,
                "description": description,
                "color": 16776960,
                "footer": {
                    "text": format!("Optima v{} | {}", env!("CARGO_PKG_VERSION"), Local::now().format("%Y-%m-%d %H:%M:%S"))
                }
            }]
        });

        let response = ureq::post(&self.config.webhook_url)
            .set("Content-Type", "application/json")
            .send_json(payload);
        
        match response {
            Ok(resp) => {
                if resp.status() == 204 {
                    self.status_message = "Sent successfully!".to_string();
                } else {
                    self.status_message = format!("Error: {}", resp.status());
                }
            }
            Err(e) => {
                self.status_message = format!("Network Error: {}", e);
            }
        }
    }
}

impl eframe::App for OptimaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Optima Dashboard");
            
            if self.last_update.elapsed().as_secs() > 2 {
                self.update_system_info();
                self.last_update = std::time::Instant::now();
            }

            let cpu_usage = self.system.global_cpu_usage();
            let mem_used = self.system.used_memory();
            let mem_total = self.system.total_memory();
            
            ui.label(format!("CPU Usage: {:.1}%", cpu_usage));
            ui.add(egui::ProgressBar::new((cpu_usage / 100.0).min(1.0)).show_percentage());
            
            ui.label(format!("RAM Used: {} / {} MB", 
                mem_used / 1024 / 1024, 
                mem_total / 1024 / 1024
            ));
            
            let mem_ratio = if mem_total > 0 {
                (mem_used as f64 / mem_total as f64) as f32
            } else {
                0.0
            };
            ui.add(egui::ProgressBar::new(mem_ratio.min(1.0)).show_percentage());

            ui.separator();
            ui.heading("Configuration");
            
            ui.text_edit_singleline(&mut self.config.api_key);
            ui.text_edit_singleline(&mut self.config.bot_token);
            ui.text_edit_singleline(&mut self.config.webhook_url);

            if ui.button("Save Configuration").clicked() {
                self.config.save();
                self.status_message = "Config saved.".to_string();
            }

            if ui.button("Test Webhook").clicked() {
                self.loading = true;
                self.status_message = "Sending...".to_string();
                // Вызываем с &mut self
                self.send_webhook("Optima Test", "This is a test message from Optima!");
                self.loading = false;
            }

            ui.label(&self.status_message);
            
            if self.loading {
                ui.spinner();
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Optima",
        native_options,
        Box::new(|_cc| Ok(Box::new(OptimaApp::new()))),
    )
}
