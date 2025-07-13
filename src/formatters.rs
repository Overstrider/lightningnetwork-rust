use chrono::{TimeZone, Utc};

const SATS_PER_BTC: f64 = 100_000_000.0;

/// Converte um valor em satoshis (i64) para uma string formatada em BTC.
pub fn format_capacity(sats: i64) -> String {
    let btc = sats as f64 / SATS_PER_BTC;
    format!("{:.8}", btc)
}

/// Converte um Unix timestamp (i64) para uma string no formato "YYYY-MM-DDTHH:MM:SSZ".
pub fn format_timestamp(ts: i64) -> String {
    // Cria um objeto DateTime<Utc> a partir do timestamp.
    let datetime = Utc.timestamp_opt(ts, 0).single();

    // Formata a data para o padrão RFC3339, que corresponde ao seu exemplo.
    if let Some(dt) = datetime {
        dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    } else {
        // Retorna um valor padrão em caso de timestamp inválido.
        "Invalid Timestamp".to_string()
    }
} 