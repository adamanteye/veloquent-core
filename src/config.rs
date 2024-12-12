//! 配置模块

use serde::Deserialize;

/// 上传配置
#[derive(Deserialize)]
pub struct Upload {
    /// 上传保存路径
    pub dir: String,
}

/// PostgreSQL 配置
#[derive(Deserialize)]
pub struct Database {
    /// 数据库名称
    pub name: String,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 主机
    pub address: String,
    /// 端口
    pub port: u16,
    /// 最大连接数
    pub max_connections: u32,
}

/// 监听配置
#[derive(Deserialize)]
pub struct Listen {
    /// 监听地址
    pub address: String,
    /// 监听端口
    pub port: u16,
}

/// 鉴权配置
#[derive(Deserialize)]
pub struct Authentication {
    /// 登陆过期时间
    pub exp_after: u64,
    /// 私钥
    pub secret: String,
}

/// 后端配置
#[derive(Deserialize)]
pub struct Config {
    /// 数据库配置
    pub database: Database,
    /// 监听配置
    pub listen: Listen,
    /// 鉴权配置
    pub authentication: Authentication,
    /// 上传配置
    pub upload: Upload,
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parse_config_file() {
        let config_file = r#"
[database]
username = "yangzheh"
password = "123456"
address = "127.0.0.1"
port = 5432
name = "veloquent"
max_connections = 10

[listen]
address = "127.0.0.1"
port = 8000

[authentication]
secret = "secret"
exp_after = 86400

[upload]
dir = "/srv/veloquent/upload"
"#;
        assert!(toml::from_str::<Config>(config_file).is_ok());
    }
}
