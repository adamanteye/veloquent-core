//! 配置模块

use serde::Deserialize;

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
    pub host: String,
    /// 端口
    pub port: u16,
    /// 最大连接数
    pub max_connections: u32,
}

/// 后端配置
#[derive(Deserialize)]
pub struct Config {
    /// 数据库配置
    pub database: Database,
}
