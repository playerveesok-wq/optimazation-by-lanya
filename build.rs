fn main() {
    if cfg!(target_os = "windows") {
        if std::path::Path::new("icon.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("icon.ico");
            res.compile().unwrap();
        } else {
            println!("cargo:warning=Файл icon.ico не найден, собираем без иконки.");
        }
    }
}
