use super::conn::get_connection; // Usamos a função para obter uma conexão do pool
use regex::Regex;
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::format;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

#[derive(Serialize)]
pub struct DatabaseInfo {
    total_rows: i32,
    date: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Lottery {
    title: Option<String>, // Torna o campo `title` opcional
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BichoDraw {
    place: Option<String>,    // Pode ser `null`
    lottery: Lottery,         // Permanece o mesmo
    thousand: Option<String>, // Pode ser `null`
    hour: Option<String>,     // Pode ser `null`
    group: Option<String>,    // Pode ser `null`
    date: Option<String>,     // Pode ser `null`
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BichoData {
    bicho_lotteries_draws: Vec<Vec<BichoDraw>>, // A estrutura permanece a mesma
    show_more: bool,
    status: String,
}

#[derive(Serialize, Deserialize)]
pub struct BichoGroup {
    id: Option<u32>,
    hour: String,
    place: u32,
    group: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LossSequence {
    pub hour: String,
    pub group: String,
    pub place: u32,
    pub loss_sequence: i32,
}

fn format_database_name(house_name: &str) -> String {
    let re = Regex::new(r"[-\s]").unwrap();
    let database_name = re.replace_all(&house_name, "_");

    database_name.to_string() // Converte de volta para String
}

pub fn get_table_info(house_name: String) -> Result<DatabaseInfo, ()> {
    let table_name = format_database_name(&house_name);
    let conn = get_connection();

    // Query para pegar a contagem e o `updated_at` mais recente
    let query = format!("SELECT count(*), MAX(updated_at) FROM {}", table_name);

    // Captura os resultados (contagem e timestamp)
    let result: rusqlite::Result<DatabaseInfo> = conn.query_row(&query, [], |row| {
        Ok(DatabaseInfo {
            total_rows: row.get(0)?,
            date: row.get(1)?,
        })
    });

    match result {
        Ok(info) => Ok(info),
        Err(rusqlite::Error::SqliteFailure(_, Some(msg))) if msg.contains("no such table") => {
            println!("Erro: A tabela '{}' não existe.", table_name);
            create_house_table_in_not_exists(&table_name).expect("Erro ao criar a tabela");
            get_table_info(house_name)
        }
        Err(err) => {
            println!("Erro inesperado: {:?}", err);
            Err(())
        }
    }
}

fn create_house_table_in_not_exists(table_name: &str) -> Result<(), rusqlite::Error> {
    let conn = get_connection();

    let query = format!(
        "CREATE TABLE IF NOT EXISTS {} (
            id INTEGER PRIMARY KEY,
            place INTEGER NOT NULL,
            date INTEGER NOT NULL,
            hour TEXT NOT NULL,
            milhar INTEGER NOT NULL,
            \"group\" INTEGER NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            UNIQUE (place, date, hour)
        )",
        table_name
    );

    conn.execute(&query, [])?;

    Ok(())
}

fn create_group_table_in_not_exists(table_name: &str) -> Result<(), rusqlite::Error> {
    let conn = get_connection();

    let query = format!(
        "CREATE TABLE IF NOT EXISTS {} (
            id INTEGER PRIMARY KEY,
            hour TEXT NOT NULL,
            place INTEGER NOT NULL,
            \"group\" TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            UNIQUE (hour, place)
        )",
        table_name
    );

    conn.execute(&query, [])?;

    Ok(())
}

pub fn save_bicho_data<F>(
    house_name: String,
    bicho_data: &str,
    mut progress_callback: F, // Callback que recebe o progresso
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(f32), // O callback recebe um f32 que representa o progresso
{
    let table_name = format_database_name(&house_name);
    match create_house_table_in_not_exists(&table_name) {
        Ok(_) => (),
        Err(err) => return Err(Box::new(err)),
    };

    let deserialized: BichoData = serde_json::from_str(&bicho_data)?;
    let total_draws = deserialized
        .bicho_lotteries_draws
        .iter()
        .map(|g| g.len())
        .sum::<usize>(); // Número total de inserções
    let mut conn = get_connection(); // Obtém conexão
    let tx = conn.transaction()?; // Inicia uma transação para as inserções

    let mut total_inserts = 0;
    let batch_size = 100; // Defina o tamanho do lote
    let mut params: Vec<String> = Vec::new();

    for draw_group in deserialized.bicho_lotteries_draws.iter() {
        for draw in draw_group.iter() {
            let position: i32 = match draw.place.as_ref().unwrap_or(&"999".to_string()).parse() {
                Ok(place) => place,
                Err(_) => continue,
            };

            let formatted_date = if let Some(date_str) = draw.date.as_ref() {
                let parts: Vec<&str> = date_str.split('/').collect();
                if parts.len() == 3 {
                    format!("{}-{}-{}", parts[2], parts[1], parts[0])
                } else {
                    "1970-01-01".to_string()
                }
            } else {
                "1970-01-01".to_string()
            };

            let insert_query = format!(
                "INSERT INTO {} (place, date, hour, milhar, \"group\", updated_at) 
                 VALUES ({}, strftime('%s', '{}'), '{}', '{}', '{}', strftime('%s', 'now'))",
                table_name,
                position,
                formatted_date,
                draw.hour.as_ref().unwrap_or(&"999".to_string()),
                draw.thousand.as_ref().unwrap_or(&"999".to_string()),
                draw.group.as_ref().unwrap_or(&"999".to_string())
            );

            // Testa se a query é válida
            match tx.execute(&insert_query, []) {
                Ok(_) => {
                    total_inserts += 1;

                    // Adiciona a query ao batch
                    params.push(insert_query);

                    // Quando atingir o batch size, executa as queries acumuladas
                    if total_inserts % batch_size == 0 {
                        // Executa cada query individualmente dentro do batch
                        for query in params.iter() {
                            if let Err(_) = tx.execute(query, []) {}
                        }
                        params.clear(); // Limpa o lote após a execução
                    }
                }
                Err(err) => {
                    println!("Erro ao validar linha: {:?}", err);
                    continue;
                }
            }

            // Chama o callback para atualizar o progresso
            let progress = (total_inserts as f32 / total_draws as f32) * 100.0;
            progress_callback(progress);
        }
    }

    // Executa as queries restantes, se houver
    if !params.is_empty() {
        for query in params.iter() {
            if let Err(err) = tx.execute(query, []) {
                println!("Erro ao inserir linha: {:?}", err);
            }
        }
    }

    // Finaliza a transação
    tx.commit()?; // Confirma as inserções válidas

    // Chama o callback com 100% de progresso ao finalizar
    progress_callback(100.0);

    Ok(())
}

pub fn export_table_to_csv(
    house_name: String,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = get_connection();
    let table_name = format_database_name(&house_name);

    // Obtém os nomes das colunas
    let stmt = conn.prepare(&format!("SELECT * FROM {} LIMIT 1", table_name))?;
    let column_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    drop(stmt); // Libera o stmt após obter os nomes das colunas

    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    // Escreve os nomes das colunas no CSV
    writeln!(writer, "{}", column_names.join(","))?;

    let mut stmt = conn.prepare(&format!("SELECT * FROM {}", table_name))?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let mut values = Vec::new();
        for idx in 0..column_names.len() {
            // Tenta converter para String e trata diferentes tipos
            let value: String = match row.get_ref(idx)? {
                rusqlite::types::ValueRef::Integer(val) => val.to_string(),
                rusqlite::types::ValueRef::Real(val) => val.to_string(),
                rusqlite::types::ValueRef::Text(val) => String::from_utf8_lossy(val).to_string(),
                rusqlite::types::ValueRef::Null => "".to_string(),
                _ => "UNSUPPORTED_TYPE".to_string(),
            };
            values.push(value);
        }

        writeln!(writer, "{}", values.join(","))?;
    }

    Ok(())
}

pub fn get_groups(house_name: String) -> Result<Vec<BichoGroup>, Box<dyn Error>> {
    let conn = get_connection();
    let table_name = String::from("group_") + &format_database_name(&house_name);

    create_group_table_in_not_exists(&table_name)?;

    let mut stmt = conn.prepare(&format!(
        "SELECT id, hour, place, \"group\" FROM {}",
        table_name
    ))?;

    // Mapeia as linhas do resultado para a estrutura BichoGroup
    let groups = stmt
        .query_map([], |row| parse_row_to_bicho_group(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(groups)
}

pub fn add_group(house_name: String, data: BichoGroup) -> Result<(), String> {
    let conn = get_connection();
    let table_name = String::from("group_") + &format_database_name(&house_name);
    let joined_str = data
        .group
        .iter()
        .map(|num| num.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let query = format!( "INSERT INTO {} (hour, place, \"group\", updated_at) VALUES ('{}', '{}', '{}', strftime('%s', 'now'))", table_name, data.hour, data.place, joined_str );
    if let Err(err) = conn.execute(&query, []) {
        if err.to_string().contains("UNIQUE constraint failed") {
            return Err(format!("o grupo das {} horas do {}º premio já está cadastrado", data.hour, data.place));
        }
        return Err(err.to_string());
    }
    Ok(())
}

pub fn edit_group(house_name: String, data: BichoGroup) -> Result<(), String> {
    if data.id == None { return  Err("Erro ao enviar ID".to_string());}
    
    let conn = get_connection();
    let table_name = String::from("group_") + &format_database_name(&house_name);

    let joined_str = data
        .group
        .iter()
        .map(|num| num.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    
    let query = format!(
        "UPDATE {} SET hour = '{}', place = '{}', \"group\" = '{}', updated_at = strftime('%s', 'now') WHERE id = {}",
        table_name, data.hour, data.place, joined_str, data.id.unwrap()
    );

    match conn.execute(&query, []) {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.to_string().contains("UNIQUE constraint failed") {
                return Err(format!("o grupo das {} horas do {}º premio já está cadastrado", data.hour, data.place));
            }
            return Err(err.to_string());
        }
    }
}

pub fn delete_group(house_name: String, id: u32) -> Result<(), String> {
    let conn = get_connection();
    let table_name = String::from("group_") + &format_database_name(&house_name);

    let query = format!("DELETE FROM {} WHERE id = {}", table_name, id);

    match conn.execute(&query, []) {
        Ok(affected_rows) => {
            if affected_rows > 0 {
                Ok(())
            } else {
                Err(format!("Nenhum grupo encontrado com o id {}", id))
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

fn parse_row_to_bicho_group(row: &Row) -> Result<BichoGroup, rusqlite::Error> {
    let id: u32 = row.get("id")?;
    let hour: String = row.get("hour")?;
    let place: u32 = row.get("place")?;
    let group_str: String = row.get("group")?;

    // Divide a string `group` e converte para um vetor de u32
    let group = group_str
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();

    Ok(BichoGroup { id: Some(id), hour, place, group })
}

pub fn get_hours(house_name: String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let conn = get_connection();
    let table_name = format_database_name(&house_name);

    let mut stmt = conn.prepare(&format!("SELECT DISTINCT hour FROM {}", table_name))?;
    let rows = stmt.query_map([], |row| row.get(0))?;

    let mut unique_values: Vec<String> = Vec::new();
    for value in rows {
        unique_values.push(value?);
    }

    Ok(unique_values)
}

pub fn get_places(house_name: String) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let conn = get_connection();
    let table_name = format_database_name(&house_name);

    let mut stmt = conn.prepare(&format!("SELECT DISTINCT place FROM {}", table_name))?;
    let rows = stmt.query_map([], |row| row.get(0))?;

    let mut unique_values: Vec<u32> = Vec::new();
    for value in rows {
        unique_values.push(value?);
    }

    Ok(unique_values)
}

pub fn get_loss_sequence(house_name: String) -> Result<Vec<LossSequence>, String> {
    let conn = get_connection();
    let table_name = format_database_name(&house_name);

    let query_groups_data = format!("SELECT hour, place, \"group\" FROM group_{}", table_name);
    let mut stmt = conn.prepare(&query_groups_data).map_err(|e| e.to_string())?;

    let groups = stmt.query_map([], |row| {
        let hour: String = row.get(0)?;
        let place: u32 = row.get(1)?;
        let group: String = row.get(2)?;
        
        Ok((hour, place, group))
    }).map_err(|e| e.to_string())?;
    
    let mut results: Vec<LossSequence> = Vec::new();
    for group_data in groups {
        let (hour, place, group) = group_data.map_err(|e| e.to_string())?;
        
        // Query para obter o último timestamp
        let query_last_occurrence = format!(
            "SELECT \"date\" FROM {} WHERE hour='{}' AND place='{}' AND \"group\" IN ({}) ORDER BY \"date\" DESC LIMIT 1",
            table_name, hour, place, group
        );
        let last_timestamp: Option<i64> = conn.query_row(&query_last_occurrence, [], |row| row.get(0)).ok();
        
        // Se não houver uma última ocorrência, usa "999" como loss_sequence
        let loss_sequence = if let Some(timestamp) = last_timestamp {
            let query_loss_sequence = format!(
                "SELECT count(*) FROM {} WHERE \"date\" > {} AND hour='{}' AND place='{}'",
                table_name, timestamp, hour, place
            );
            
            conn.query_row(&query_loss_sequence, [], |row| row.get(0)).unwrap_or(999)
        } else {
            999
        };

        results.push(LossSequence {
            hour,
            place,
            group,
            loss_sequence,
        });
    }

    Ok(results)
}


#[cfg(test)]
mod testes {
    use super::get_loss_sequence;


    fn test_get_loss_sequence() {
        let a = get_loss_sequence("A Zebra".to_owned()).unwrap();
        println!("saida: {:?}", a);
    }
}