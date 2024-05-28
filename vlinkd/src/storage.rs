use std::io;
use anyhow::anyhow;
use directories::ProjectDirs;
use log::info;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vlink_core::secret::VlinkStaticSecret;
use crate::config::{ StorageConfig};

pub struct Storage {
    pub path: Option<String>,
}

impl Storage {
    pub async fn load_config(&self) -> anyhow::Result<StorageConfig> {
        let cp =match &self.path {
            None => {
                let proj_dirs = ProjectDirs::from("cn", "hperfect", "vlink")
                    .ok_or(anyhow!("配置目录打开错误"))?;
                proj_dirs.config_dir().to_path_buf()
            }
            Some(s) => {
                s.into()
            }
        };
        //读取配置文件
        let key = cp.join("config.json");
        let file = File::open(key.as_path()).await;

        return match file {
            Ok(mut f) => {
                let mut pvk = String::new();
                f.read_to_string(&mut pvk).await?;
                let cfg: StorageConfig = serde_json::from_str(pvk.as_str())?;
                Ok(cfg)
            }
            Err(_) => {
                fs::create_dir_all(cp).await?;
                //创建文件
                let mut file = File::create(key.as_path()).await?;
                // 生成秘钥对写入
                let config = StorageConfig {
                    secret: VlinkStaticSecret::generate(),
                };
                let txt = serde_json::to_string(&config)?;
                file.write_all(txt.as_bytes()).await?;
                info!("初始化key:{},\n存储路径:{:?}",config.secret.base64_pub().as_str(),key.as_os_str());
                Ok(config)
            }
        };
        // ProjectDirs::from
    }
}

