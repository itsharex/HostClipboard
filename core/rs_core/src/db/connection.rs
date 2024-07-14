use migration::Migrator;
use migration::MigratorTrait;
use sea_orm::{Database, DatabaseConnection, DbErr};

pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
  let db = Database::connect(database_url).await;
  match db {
    Ok(conn) => {
      Migrator::up(&conn, None).await?;
      return Ok(conn);
    }
    Err(e) => return Err(e),
  }
}
