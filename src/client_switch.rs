use crate::hutao_config::ASSETS_PATH;
use std::fs::read_to_string;
use std::io::Read;
use std::path::Path;

#[derive(PartialEq, Clone, Copy)]
pub enum ClientType {
    Official,
    Bilibili,
}

pub struct ClientSwitch {
    pub game_path: String,
    pub client_type: ClientType,
}

impl Default for ClientSwitch {
    fn default() -> Self {
        Self {
            game_path: read_to_string(format!("{ASSETS_PATH}/game_path.txt")).unwrap_or_else(
                |_| {
                    "D:\\Program Files\\Genshin Impact\\Genshin Impact Game\\YuanShen.exe"
                        .to_string()
                },
            ),
            client_type: ClientType::Official,
        }
    }
}

impl ClientSwitch {
    pub fn switch(&mut self) -> Result<(), String> {
        let exe_path = self.game_path.trim();
        let game_dir = Path::new(exe_path).parent().ok_or("Invalid game path")?;
        let config_path = game_dir.join("config.ini");

        // config.ini
        let mut config_content = String::new();
        {
            let mut file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&config_path)
                .map_err(|_| format!("Cannot open config.ini: {}", config_path.display()))?;

            file.read_to_string(&mut config_content)
                .map_err(|_| "Failed to read config.ini")?;
        } // auto close file
        let mut lines: Vec<&str> = config_content.lines().collect();
        for line in lines.iter_mut() {
            if line.starts_with("channel=") {
                *line = match self.client_type {
                    ClientType::Official => "channel=1",
                    ClientType::Bilibili => "channel=14",
                };
            }
        }
        let new_content = lines.join("\n");
        std::fs::write(&config_path, new_content).map_err(|_| "Failed to write config.ini")?;

        if self.client_type == ClientType::Bilibili {
            let sdk_path = game_dir
                .join("YuanShen_Data")
                .join("Plugins")
                .join("PCGameSDK.dll");
            let pkg_path = game_dir.join("sdk_pkg_version");
            let path_string = format!("{ASSETS_PATH}/switch");
            let assets_dir = Path::new(&path_string);

            if !sdk_path.exists() {
                let src_sdk = assets_dir
                    .join("YuanShen_Data")
                    .join("Plugins")
                    .join("PCGameSDK.dll");
                std::fs::copy(src_sdk, sdk_path).map_err(|_| "Failed to copy PCGameSDK.dll")?;
            }
            if !pkg_path.exists() {
                let src_pkg = assets_dir.join("sdk_pkg_version");
                std::fs::copy(src_pkg, pkg_path).map_err(|_| "Failed to copy sdk_pkg_version")?;
            }
        }
        Ok(())
    }
}
