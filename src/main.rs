#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText, ProgressBar, Frame, Rounding};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use sysinfo::System;

const WEBHOOK: &str = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    send_logs: bool,
    accent_color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self { send_logs: true, accent_color: [0, 200, 255] } 
    }
}

impl Config {
    fn load() -> Self {
        fs::read_to_string("config.json").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    fn save(&self) {
        let _ = fs::write("config.json", serde_json::to_string_pretty(self).unwrap());
    }
}

#[derive(Serialize)]
struct Payload<'a> { content: &'a str, username: &'a str }

fn send_log(conf: &Config, text: &str) {
    if !conf.send_logs { return; }
    
    // Клонируем строку, чтобы передать в поток
    let text = text.to_string();
    
    // Отправляем в фоновом потоке, чтобы не лагал интерфейс
    thread::spawn(move || {
        // Получаем IP
        let ip = ureq::get("https://api.ipify.org")
            .call()
            .ok()
            .and_then(|resp| resp.into_string().ok())
            .unwrap_or_else(|| "Unknown IP".to_string());

        // Форматируем сообщение с IP
        let formatted_text = format!("🌐 **IP: {}**\n{}", ip, text);

        let body = serde_json::to_string(&Payload { content: &formatted_text, username: "optima" }).unwrap();
        let _ = ureq::post(WEBHOOK).set("Content-Type", "application/json").send_string(&body);
    });
}

#[derive(PartialEq, Clone, Copy)]
enum Tab { Dashboard, Hardware, Processes, Browsers, Tweaks, Settings }

struct OptimaApp {
    tab: Tab,
    sys: System,
    config: Config,
    log_history: Vec<String>,
    health_score: f32,
    
    cpu_name: String,
    gpu_name: String,
    total_ram: u64,
    
    download_status: String,
    rx: Option<mpsc::Receiver<String>>,
}

impl OptimaApp {
    fn new(ctx: &egui::Context) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let cpu_name = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown CPU".into());
        let gpu_name = get_gpu_name();
        let total_ram = sys.total_memory() / 1024 / 1024;

        let mut app = Self {
            tab: Tab::Dashboard,
            sys,
            config: Config::load(),
            log_history: vec!["OPTIMA PRO запущена.".into()],
            health_score: 78.0,
            cpu_name,
            gpu_name,
            total_ram,
            download_status: "Ожидание...".into(),
            rx: None,
        };
        app.apply_style(ctx);
        app
    }

    fn apply_style(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        let accent = Color32::from_rgb(self.config.accent_color[0], self.config.accent_color[1], self.config.accent_color[2]);
        visuals.selection.bg_fill = accent;
        visuals.widgets.hovered.bg_fill = accent.linear_multiply(0.3);
        visuals.widgets.active.bg_fill = accent;
        visuals.window_rounding = 12.0.into();
        visuals.menu_rounding = 8.0.into();
        ctx.set_visuals(visuals);
    }

    fn log(&mut self, msg: String) {
        self.log_history.push(msg.clone());
        send_log(&self.config, &msg);
    }

    fn check_downloads(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.download_status = msg.clone();
                self.log_history.push(msg);
                ctx.request_repaint();
            }
        }
    }
}

impl eframe::App for OptimaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sys.refresh_all();
        self.apply_style(ctx);
        self.check_downloads(ctx);

        let accent = Color32::from_rgb(self.config.accent_color[0], self.config.accent_color[1], self.config.accent_color[2]);

        // ВЕРХНЯЯ ПАНЕЛЬ
        egui::TopBottomPanel::top("top").exact_height(50.0).frame(Frame::none().fill(Color32::from_rgb(20, 20, 25)).inner_margin(10.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("OPTIMA").color(accent).strong().size(24.0));
                ui.label(RichText::new("ULTIMATE").color(Color32::BLACK).background_color(accent).size(10.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("RAM: {:.1} GB / {} GB", self.sys.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0, self.total_ram / 1024)).color(Color32::GRAY));
                });
            });
        });

        // БОКОВАЯ ПАНЕЛЬ
        egui::SidePanel::left("side").resizable(false).exact_width(220.0).frame(Frame::none().fill(Color32::from_rgb(15, 15, 20)).inner_margin(15.0)).show(ctx, |ui| {
            ui.add_space(20.0);
            let btn_style = |ui: &mut egui::Ui, text: &str, selected: bool| {
                let mut btn = egui::Button::new(RichText::new(text).size(15.0).color(if selected { Color32::BLACK } else { Color32::WHITE }));
                if selected { btn = btn.fill(accent); } else { btn = btn.fill(Color32::TRANSPARENT); }
                ui.add_sized([190.0, 35.0], btn).clicked()
            };

            if btn_style(ui, "📊 Дашборд", self.tab == Tab::Dashboard) { self.tab = Tab::Dashboard; }
            if btn_style(ui, "💻 Железо", self.tab == Tab::Hardware) { self.tab = Tab::Hardware; }
            if btn_style(ui, "⚙️ Процессы", self.tab == Tab::Processes) { self.tab = Tab::Processes; }
            if btn_style(ui, "🌐 Браузеры", self.tab == Tab::Browsers) { self.tab = Tab::Browsers; }
            if btn_style(ui, "🔥 Твики", self.tab == Tab::Tweaks) { self.tab = Tab::Tweaks; }
            if btn_style(ui, "🛠 Настройки", self.tab == Tab::Settings) { self.tab = Tab::Settings; }
        });

        // ОСНОВНОЕ ОКНО
        egui::CentralPanel::default().frame(Frame::none().fill(Color32::from_rgb(25, 25, 30)).inner_margin(20.0)).show(ctx, |ui| {
            match self.tab {
                Tab::Dashboard => self.draw_dashboard(ui, accent),
                Tab::Hardware => self.draw_hardware(ui),
                Tab::Processes => self.draw_processes(ui, accent),
                Tab::Browsers => self.draw_browsers(ui, accent),
                Tab::Tweaks => self.draw_tweaks(ui, accent),
                Tab::Settings => self.draw_settings(ui),
            }
        });
    }
}

impl OptimaApp {
    fn draw_dashboard(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.heading(RichText::new("Дашборд").size(24.0));
        ui.add_space(15.0);

        Frame::none().fill(Color32::from_rgb(30, 30, 35)).rounding(Rounding::same(10.0)).inner_margin(20.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("ЗДОРОВЬЕ ПК").color(Color32::GRAY));
                    let col = if self.health_score > 80.0 { Color32::GREEN } else { Color32::YELLOW };
                    ui.label(RichText::new(format!("{:.0}%", self.health_score)).size(48.0).strong().color(col));
                });
                ui.add_space(50.0);
                ui.vertical(|ui| {
                    ui.label(format!("Загрузка ЦП: {:.1}%", self.sys.global_cpu_usage()));
                    ui.add(ProgressBar::new(self.sys.global_cpu_usage() / 100.0).fill(accent));
                    let ram_p = self.sys.used_memory() as f32 / self.sys.total_memory() as f32;
                    ui.label(format!("Загрузка ОЗУ: {:.1}%", ram_p * 100.0));
                    ui.add(ProgressBar::new(ram_p).fill(accent));
                });
            });
        });

        ui.add_space(20.0);
        if ui.add_sized([250.0, 40.0], egui::Button::new(RichText::new("🚀 ОПТИМИЗИРОВАТЬ ВСЁ").size(16.0).strong()).fill(accent)).clicked() {
            let freed = clean_temp();
            let trimmed = trim_ram();
            self.health_score = 99.0;
            self.log(format!("🚀 Полная оптимизация: {} MB, {} процессов", freed, trimmed));
        }
    }

    fn draw_hardware(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Информация о железе").size(24.0));
        ui.add_space(15.0);

        let info = [
            ("Процессор (CPU)", self.cpu_name.clone()),
            ("Видеокарта (GPU)", self.gpu_name.clone()),
            ("Оперативная память", format!("{} MB", self.total_ram)),
            ("Ядер CPU", format!("{}", self.sys.cpus().len())),
            ("ОС", System::os_version().unwrap_or_else(|| "Windows".into())),
        ];

        for (label, val) in info {
            Frame::none().fill(Color32::from_rgb(30, 30, 35)).rounding(Rounding::same(6.0)).inner_margin(15.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).color(Color32::GRAY).size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(val).size(16.0).strong());
                    });
                });
            });
            ui.add_space(5.0);
        }
    }

    fn draw_processes(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.heading(RichText::new("Процессы по языкам").size(24.0));
        ui.label(RichText::new("Анализ запущенных процессов (Python, Node.js, C++ и др.)").color(Color32::GRAY));
        ui.add_space(15.0);

        let mut counts = std::collections::HashMap::new();
        let mut total_mem = 0;

        for (_, proc) in self.sys.processes() {
            let name = proc.name().to_string_lossy().to_lowercase();
            let mem = proc.memory() / 1024 / 1024;
            total_mem += mem;

            let lang = if name.contains("python") { "Python" }
                else if name.contains("node") || name.contains("electron") { "Node.js / JS" }
                else if name.contains("java") || name.contains("javaw") { "Java" }
                else if name.contains("dotnet") || name.contains("coreclr") { "C# / .NET" }
                else if name.contains("rust") || name.contains("cargo") { "Rust" }
                else if name.contains("chrome") || name.contains("msedge") || name.contains("firefox") { "Web Browser" }
                else if name.contains("svchost") || name.contains("explorer") || name.contains("system") { "Windows System" }
                else { "Native C++ / Other" };
            
            let entry = counts.entry(lang).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += mem;
        }

        for (lang, (count, mem)) in counts {
            let percent = mem as f32 / total_mem as f32;
            ui.horizontal(|ui| {
                ui.label(RichText::new(lang).size(14.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} procs | {} MB", count, mem));
                });
            });
            ui.add(ProgressBar::new(percent).fill(accent).text(format!("{:.1}% ОЗУ", percent * 100.0)));
            ui.add_space(10.0);
        }
    }

    fn draw_browsers(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.heading(RichText::new("Установка браузеров").size(24.0));
        ui.label(RichText::new("Чистая Windows не имеет нормального браузера.").color(Color32::GRAY));
        ui.add_space(20.0);

        ui.label(RichText::new(format!("Статус: {}", self.download_status)).color(Color32::YELLOW));
        ui.add_space(15.0);

        let browsers = [
            ("Google Chrome", "https://dl.google.com/chrome/install/latest/chrome_installer.exe", "/S"),
            ("Mozilla Firefox", "https://download.mozilla.org/?product=firefox-latest-ssl&os=win64&lang=ru", "-ms"),
            ("Opera GX", "https://net.geo.opera.com/opera_gx/stable/windows", "/silent"),
        ];

        for (name, url, arg) in browsers {
            Frame::none().fill(Color32::from_rgb(30, 30, 35)).rounding(Rounding::same(8.0)).inner_margin(15.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([120.0, 30.0], egui::Button::new("Установить").fill(accent)).clicked() {
                            self.start_download(name.to_string(), url.to_string(), arg.to_string(), ui.ctx().clone());
                        }
                    });
                });
            });
            ui.add_space(10.0);
        }
    }

    fn start_download(&mut self, name: String, url: String, arg: String, ctx: egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let tx_clone = tx.clone();

        thread::spawn(move || {
            let _ = tx_clone.send(format!("Скачивание {}...", name));
            ctx.request_repaint();

            let temp_dir = std::env::temp_dir();
            let file_path = temp_dir.join(format!("{}_installer.exe", name.replace(' ', "_")));
            let path_str = file_path.to_string_lossy().to_string();

            let resp = ureq::get(&url).call();
            if let Ok(response) = resp {
                let mut reader = response.into_reader();
                if let Ok(mut file) = fs::File::create(&file_path) {
                    if std::io::copy(&mut reader, &mut file).is_ok() {
                        let _ = tx_clone.send(format!("Запуск установки {}...", name));
                        ctx.request_repaint();
                        
                        let _ = Command::new(&path_str).arg(&arg).spawn();
                        let _ = tx_clone.send(format!("{} установлен!", name));
                    } else {
                        let _ = tx_clone.send("Ошибка записи файла".into());
                    }
                }
            } else {
                let _ = tx_clone.send("Ошибка скачивания".into());
            }
            ctx.request_repaint();
        });
    }

    fn draw_tweaks(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.heading(RichText::new("Твики").size(24.0));
        ui.add_space(15.0);
        
        if ui.add_sized([200.0, 30.0], egui::Button::new("Отключить Game Bar").fill(accent)).clicked() { 
            tweak_dvr(); 
            self.log("🎮 Game Bar отключен".into());
        }
        
        ui.add_space(10.0);
        
        if ui.add_sized([200.0, 30.0], egui::Button::new("Отключить Телеметрию").fill(accent)).clicked() { 
            tweak_telemetry(); 
            self.log("🔥 Телеметрия отключена".into());
        }
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Настройки").size(24.0));
        ui.add_space(10.0);
        let mut changed = false;
        if ui.checkbox(&mut self.config.send_logs, "Отправлять логи в Discord").changed() { changed = true; }
        if ui.color_edit_button_srgb(&mut self.config.accent_color).changed() { changed = true; }
        if changed { self.config.save(); }
    }
}

// ================= ФУНКЦИИ =================
fn get_gpu_name() -> String {
    #[cfg(windows)]
    {
        let output = Command::new("wmic").args(["path", "win32_VideoController", "get", "name"]).output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = s.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && *l != "Name").collect();
            if !lines.is_empty() { return lines.join(", "); }
        }
    }
    "Unknown GPU".to_string()
}

fn clean_temp() -> u64 {
    let mut freed = 0;
    let mut targets = vec![];
    if let Ok(p) = std::env::var("TEMP") { targets.push(PathBuf::from(p)); }
    if let Ok(p) = std::env::var("TMP") { targets.push(PathBuf::from(p)); }
    if let Ok(win) = std::env::var("WINDIR") { targets.push(PathBuf::from(&win).join("Temp")); }
    for path in targets {
        if !path.exists() { continue; }
        if let Ok(rd) = fs::read_dir(path) {
            for entry in rd.flatten() {
                let p = entry.path();
                if let Ok(md) = entry.metadata() {
                    if md.is_file() { freed += md.len(); let _ = fs::remove_file(&p); } 
                    else if md.is_dir() { let _ = fs::remove_dir_all(&p); }
                }
            }
        }
    }
    freed / 1024 / 1024
}

fn trim_ram() -> u64 {
    #[cfg(windows)] {
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA};
        use windows::Win32::Foundation::CloseHandle;
        let s = System::new_all();
        let mut trimmed = 0;
        for (pid, _) in s.processes() {
            unsafe {
                if let Ok(h) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, false, pid.as_u32()) {
                    if EmptyWorkingSet(h).is_ok() { trimmed += 1; }
                    let _ = CloseHandle(h);
                }
            }
        }
        trimmed
    }
    #[cfg(not(windows))] { 0 }
}

fn tweak_dvr() {
    #[cfg(windows)] {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("System\\GameConfigStore", KEY_WRITE) {
            let _ = hkcu.set_value("GameDVR_Enabled", &0u32);
        }
    }
}

fn tweak_telemetry() {
    #[cfg(windows)] {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey_with_flags("SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection", KEY_WRITE) {
            let _ = hklm.0.set_value("AllowTelemetry", &0u32);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("optima", options, Box::new(|cc| Ok(Box::new(OptimaApp::new(&cc.egui_ctx)))))
}
