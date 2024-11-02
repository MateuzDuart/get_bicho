use modules::db::{BichoGroup, LossSequence};
use tauri::{AppHandle, Emitter};

mod modules;

#[tauri::command]
async fn houses() -> String {
    let houses = modules::web::get_houses()
        .await
        .expect("erro ao pegar as casas");

    let json = serde_json::to_string(&houses).expect("Erro ao serializar o json");

    json
}

#[tauri::command]
async fn get_database_info(house_name: String) -> Result<String, ()> {
    match modules::db::get_table_info(house_name) {
        Ok(database_info) => {
            let json =
                serde_json::to_string(&database_info).expect("Erro ao serializar o database info");

            return Ok(json);
        }
        Err(_) => {
            return Err(());
        }
    }
}

#[tauri::command]
async fn get_bichos_data(
    app_handle: AppHandle,
    house_name: String,
    lottery: String,
    total_days: i32,
) -> Result<String, String> {
    println!("{}", total_days);
    match modules::web::get_bichos_data(lottery, total_days).await {
        Ok(json) => {
            match modules::db::save_bicho_data(house_name, &json, |progress| {
                app_handle.emit("progress", progress).expect("Falha ao emitir evento");
            }) {
                Ok(_) => {}
                Err(err) => {
                    println!("Erro salvar data: {:?}", err);
                    return Err(err.to_string());
                }
            }

            return Ok("Dados salvado com sucesso".to_string());
        }
        Err(err) => {
            println!("Erro pegar data: {:?}", err);
            return Err(err.to_string());
        }
    }
}

#[tauri::command]
fn export_csv(house_name: String, file_path: String) -> Result<(), String> {
    match modules::db::export_table_to_csv(house_name, &file_path) {
        Ok(_) => {}
        Err(err) => {
            return Err(err.to_string());
        }
    }

    Ok(())
}

#[tauri::command]
fn get_group(house_name: String) -> Result<String, String> {
    let groups = match modules::db::get_groups(house_name) {
        Ok(hours) => hours,
        Err(err) => {
            println!("{:?}", err);
            return Err(err.to_string());
        },
    };

    match serde_json::to_string(&groups) {
        Ok(json) => Ok(json),
        Err(err) => Err(format!("Erro ao serializar o JSON: {}", err)),
    }
}

#[tauri::command]
fn get_hours(house_name: String) -> Result<Vec<String>, String> {
    match modules::db::get_hours(house_name) {
        Ok(hours) => Ok(hours),
        Err(err) => Err(err.to_string())
    }
}

#[tauri::command]
fn get_places(house_name: String) -> Result<Vec<u32>, String> {
    match modules::db::get_places(house_name) {
        Ok(places) => Ok(places),
        Err(err) => Err(err.to_string())
    }
}

#[tauri::command]
fn add_group(house_name: String, data: BichoGroup) -> Result<(), String> {
    modules::db::add_group(house_name, data)?;

    Ok(())
}

#[tauri::command]
fn edit_group(house_name: String, data: BichoGroup) -> Result<(), String> {
    modules::db::edit_group(house_name, data)?;

    Ok(())
}

#[tauri::command]
fn delete_group(house_name: String, id: u32) -> Result<(), String> {
    modules::db::delete_group(house_name, id)?;

    Ok(())
}

#[tauri::command]
fn get_loss_sequence(house_name: String) -> Result<Vec<LossSequence>, String> {
    let loss_sequence = modules::db::get_loss_sequence(house_name)?;

    Ok(loss_sequence)
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            houses,
            get_database_info,
            get_bichos_data,
            export_csv,
            get_hours,
            get_places,
            get_group,
            add_group,
            edit_group,
            delete_group,
            get_loss_sequence,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
