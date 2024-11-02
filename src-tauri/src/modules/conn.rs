use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Result;
use std::fs;

// Inicializa a conexão com pool
lazy_static::lazy_static! {
    static ref POOL: Pool<SqliteConnectionManager> = {
        let manager = SqliteConnectionManager::file("bicho.db");
        Pool::new(manager).expect("Falha ao criar o pool de conexões")
    };
}

// Função para retornar a conexão do pool
pub fn get_connection() -> PooledConnection<SqliteConnectionManager> {
    POOL.get().expect("Falha ao obter conexão do pool")
}

// Função para inicializar o banco de dados, caso necessário
pub fn initialize_db() -> Result<()> {
    let db_path = "bicho.db";

    if !db_exists(db_path) {
        println!("Banco de dados não encontrado. Criando novo banco...");
        let conn = get_connection();
    } else {
        println!("Banco de dados encontrado. Conectando...");
    }

    Ok(())
}

// Verifica se o banco de dados já existe
fn db_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
