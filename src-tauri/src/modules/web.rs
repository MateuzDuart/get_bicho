use chrono::Utc;
use lazy_static::lazy_static;
use reqwest;
use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct House {
    pub name: String,
    pub value: String,
}

lazy_static! {
    static ref CACHE: Mutex<Option<(Vec<House>, Instant)>> = Mutex::new(None);
}

const CACHE_DURATION: Duration = Duration::from_secs(60 * 60 * 3); // 5 minutos

pub async fn get_houses() -> Result<Vec<House>, Box<dyn Error>> {
    // Verifica o cache
    let mut cache = CACHE.lock().await;
    if let Some((cached_houses, timestamp)) = &*cache {
        if timestamp.elapsed() < CACHE_DURATION {
            // Retorna o cache se ainda estiver válido
            return Ok(cached_houses.clone());
        }
    }

    // Realiza a requisição HTTP e coleta os dados se o cache expirou
    let url = "https://bicho365.com/deu-no-poste";
    let body = reqwest::get(url).await?.text().await?;
    let document = Html::parse_document(&body);
    let select_selector = Selector::parse("select[onchange]").unwrap();

    let mut houses: Vec<House> = Vec::new();
    if let Some(first_element) = document.select(&select_selector).next() {
        let option_selector = Selector::parse("option").unwrap();
        for option in first_element.select(&option_selector) {
            let text = option.inner_html();
            let clean_text = text.trim();
            let value = option.value().attr("value").unwrap_or("N/A");
            if value == "N/A" {
                continue;
            }

            houses.push(House {
                name: clean_text.to_string(),
                value: value.to_string(),
            });
        }
    }

    // Atualiza o cache com os dados novos
    *cache = Some((houses.clone(), Instant::now()));
    Ok(houses)
}

pub async fn get_bichos_data(lottery: String, total_days: i32) -> Result<String, Box<dyn Error>> {
    println!("a: {}", total_days);
    if total_days == 0 {
        println!("sem dados a pegar");
        return Err("não há atualizações pendentes".into());
    }
    let current_date = Utc::now().format("%Y-%m-%d").to_string();
    let url = format!(
        "https://bicho365.com/wp-content/themes/os-bicho365-child/ajax/lottery-results-archive.php?wp_site_id=1&wp_post_id=323&data%5Bfields%5D%5Blottery%5D={}&data%5Bfields%5D%5Bdraw_type%5D=&data%5Bfields%5D%5Bdatetime%5D={}&data%5Bdisplay%5D%5B%5D={}&data%5Bdisplay%5D%5B%5D=10",
        lottery,
        current_date,
        100 * total_days
    );

    let body = reqwest::get(url).await?.text().await?;
    Ok(body)
}
