pub use sea_orm_migration::*;

mod m20241012_000001_create_table_user;

/// 数据库定义
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20241012_000001_create_table_user::Migration)]
    }
}
