pub use sea_orm_migration::*;

mod m20241012_000001_create_table_user;
mod m20241014_000002_create_table_upload;
mod m20241028_000003_create_table_contact;
mod m20241110_000004_create_table_session;

/// 数据库定义
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241012_000001_create_table_user::Migration),
            Box::new(m20241014_000002_create_table_upload::Migration),
            Box::new(m20241028_000003_create_table_contact::Migration),
            Box::new(m20241110_000004_create_table_session::Migration),
        ]
    }
}
