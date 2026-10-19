//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Connessioni: `connect` apre il provider giusto a partire da una URL e
//! mette a disposizione l'estrazione del nome tabella dalle query.

use crate::data::error::DataError;
use crate::data::memory::connect_memory;
use crate::data::provider::DataProvider;

/// Apre un provider dati a partire da una URL di connessione.
///
/// Formati supportati:
/// - `memory:` (o `memory`) — `MemoryProvider`,nessun database vero e proprio;
/// - `sqlite:/percorso/db.sqlite`, `sqlite:///percorso/assoluto`,
///   `sqlite::memory:` — SQLite (feature `data-sqlite`);
/// - `mysql://user:pass@host:port/db` e `mariadb://…` — MySQL / MariaDB
///   (feature `data-mysql`); la variante MySQL o MariaDB viene rilevata a
///   runtime con `SELECT VERSION()`;
/// - un semplice percorso file (es. `dati/app.db`) — trattato come SQLite.
///
/// La query string (`?…`) e il frammento (`#…`) delle URL MySQL vengono
/// ignorati: per le opzioni avanzate si costruisce `mysql::Opts` a mano e
/// si chiama `MySqlProvider::connect`.
///
/// API sincrona (nessun runtime async) e mai `unwrap` sugli errori: tutto
/// confluisce in `DataError`.
///
/// # Esempio
/// ```no_run
/// use ru_wx::data::{connect, DataValue};
///
/// let mut db = connect("sqlite::memory:").unwrap();
/// db.execute("CREATE TABLE clienti (id INTEGER PRIMARY KEY, nome TEXT)", &[])
///     .unwrap();
/// db.execute("INSERT INTO clienti (nome) VALUES (?)", &[DataValue::Text("Mario".into())])
///     .unwrap();
/// let tabella = db.query("SELECT id, nome FROM clienti", &[]).unwrap();
/// let (headers, rows) = tabella.to_string_matrix();
/// ```
pub fn connect(url: &str) -> Result<Box<dyn DataProvider>, DataError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(DataError::connect("URL di connessione vuota"));
    }

    // Provider in memoria: nessun backend SQL.
    if trimmed.eq_ignore_ascii_case("memory") || starts_with_ci(trimmed, "memory:") {
        return Ok(Box::new(connect_memory()));
    }

    // SQLite (`sqlite:`, `sqlite::memory:`, `sqlite:///…`).
    if starts_with_ci(trimmed, "sqlite:") {
        let path = trimmed.get(7..).unwrap_or("");
        return connect_sqlite(path);
    }

    // MySQL / MariaDB (`mysql://…`, `mariadb://…`).
    if trimmed.contains("://") {
        if starts_with_ci(trimmed, "mysql://") || starts_with_ci(trimmed, "mariadb://") {
            return connect_mysql(trimmed);
        }
        return Err(DataError::connect(format!(
            "scheme non riconosciuto in \"{trimmed}\" \
             (attesi: sqlite:, mysql://, mariadb://, memory:)"
        )));
    }

    // Percorso semplice (es. `dati/app.db`): trattato come file SQLite.
    connect_sqlite(trimmed)
}

/// Apre un provider SQLite dal percorso (`:memory:` apre in memoria).
#[cfg(feature = "data-sqlite")]
fn connect_sqlite(path: &str) -> Result<Box<dyn DataProvider>, DataError> {
    use crate::data::sqlite::SqliteProvider;

    let mut p = path.trim();
    // `sqlite://relativo.db` e `sqlite:///assoluto.db`: via il prefisso `//`.
    if let Some(rest) = p.strip_prefix("//") {
        p = rest;
    }
    // `sqlite:///C:/file.db` (Windows): via lo slash residuo prima dell'unità.
    if p.len() > 3
        && p.starts_with('/')
        && p.as_bytes()[1].is_ascii_alphabetic()
        && p.as_bytes()[2] == b':'
    {
        p = &p[1..];
    }

    let provider = if p == ":memory:" {
        SqliteProvider::open_in_memory()?
    } else {
        SqliteProvider::open(p)?
    };
    Ok(Box::new(provider))
}

/// Variante senza la feature `data-sqlite`: rifiuta con un errore chiaro.
#[cfg(not(feature = "data-sqlite"))]
fn connect_sqlite(_path: &str) -> Result<Box<dyn DataProvider>, DataError> {
    Err(DataError::Unsupported(
        "supporto SQLite: abilitare la feature cargo `data-sqlite`",
    ))
}

/// Apre una connessione MySQL / MariaDB dalla URL.
#[cfg(feature = "data-mysql")]
fn connect_mysql(url: &str) -> Result<Box<dyn DataProvider>, DataError> {
    use crate::data::mysql::MySqlProvider;

    let opts = parse_mysql_url(url)?;
    let provider = MySqlProvider::connect(opts)?;
    Ok(Box::new(provider))
}

/// Variante senza la feature `data-mysql`: rifiuta con un errore chiaro.
#[cfg(not(feature = "data-mysql"))]
fn connect_mysql(_url: &str) -> Result<Box<dyn DataProvider>, DataError> {
    Err(DataError::Unsupported(
        "supporto MySQL/MariaDB: abilitare la feature cargo `data-mysql`",
    ))
}

/// Converte una URL `mysql://` / `mariadb://` in `mysql::Opts` (parsing a
/// mano, senza dipendenze aggiuntive).
///
/// Gestisce `user:password@host:port/db`, host IPv6 tra parentesi quadre
/// (`[::1]:3306`) e percent-encoding (`%40` → `@`) nei dati accesso.
/// Query string e frammento vengono ignorati.
#[cfg(feature = "data-mysql")]
fn parse_mysql_url(url: &str) -> Result<mysql::Opts, DataError> {
    let bad = |msg: String| DataError::connect(format!("URL MySQL non valida: {msg}"));

    let scheme_end = url
        .find("://")
        .ok_or_else(|| bad(format!("manca \"://\" in \"{url}\"")))?;
    let scheme = &url[..scheme_end];
    if !scheme.eq_ignore_ascii_case("mysql") && !scheme.eq_ignore_ascii_case("mariadb") {
        return Err(bad(format!("scheme \"{scheme}\" non valido")));
    }
    let body = &url[scheme_end + 3..];

    // Via query string e frammento (le opzioni avanzate passano da Opts).
    let body = body.split(['?', '#']).next().unwrap_or(body);

    // `authority/db`: il database è tutto ciò che segue il primo `/`.
    let (authority, db_path) = match body.find('/') {
        Some(pos) => (&body[..pos], Some(&body[pos + 1..])),
        None => (body, None),
    };
    let db_name = db_path
        .map(|d| {
            let d = d.split('/').next().unwrap_or(d);
            percent_decode(d)
        })
        .filter(|d| !d.is_empty());

    // Dati accesso opzionali prima dell'`@` (l'ultimo `@` separa dall'host).
    let mut user: Option<String> = None;
    let mut pass: Option<String> = None;
    let mut host_port = authority;
    if let Some(at) = authority.rfind('@') {
        let userinfo = &authority[..at];
        host_port = &authority[at + 1..];
        match userinfo.find(':') {
            Some(colon) => {
                user = Some(percent_decode(&userinfo[..colon]));
                pass = Some(percent_decode(&userinfo[colon + 1..]));
            }
            None => user = Some(percent_decode(userinfo)),
        }
    }

    // Host e porta (con supporto IPv6 tra parentesi quadre).
    let (host, port) =
        split_host_port(host_port).ok_or_else(|| bad(format!("host non valido in \"{url}\"")))?;

    let mut builder = mysql::OptsBuilder::new().ip_or_hostname(Some(host));
    if let Some(u) = user {
        builder = builder.user(Some(u));
    }
    if let Some(p) = pass {
        builder = builder.pass(Some(p));
    }
    if let Some(p) = port {
        builder = builder.tcp_port(p);
    }
    if let Some(db) = db_name {
        builder = builder.db_name(Some(db));
    }
    Ok(builder.into())
}

/// Divide `host:port` nelle sue componenti; `None` solo se la stringa è vuota.
///
/// Formati ammessi: `host`, `host:3306`, `[::1]`, `[::1]:3306`.
#[cfg(feature = "data-mysql")]
fn split_host_port(host_port: &str) -> Option<(String, Option<u16>)> {
    if host_port.is_empty() {
        return None;
    }
    if let Some(rest) = host_port.strip_prefix('[') {
        // IPv6: `[::1]:3306`
        let end = rest.find(']')?;
        let host = rest[..end].to_string();
        let tail = &rest[end + 1..];
        let port = match tail.strip_prefix(':') {
            Some(p) => Some(p.parse::<u16>().ok()?),
            None if tail.is_empty() => None,
            None => return None,
        };
        return Some((host, port));
    }
    match host_port.rfind(':') {
        Some(pos) => {
            let host = host_port[..pos].to_string();
            let port = host_port[pos + 1..].parse::<u16>().ok()?;
            Some((host, Some(port)))
        }
        None => Some((host_port.to_string(), None)),
    }
}

/// Decodifica percent-encoding (`%40` → `@`); se non ci sono sequenze
/// valide restituisce la stringa originale.
#[cfg(feature = "data-mysql")]
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 3 <= bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Confronto dei primi byte di una stringa ignorando maiuscole/minuscole
/// (i prefissi delle URL sono ASCII, quindi i tagli a byte sono sicuri).
fn starts_with_ci(haystack: &str, prefix: &str) -> bool {
    match haystack.get(..prefix.len()) {
        Some(head) => head.eq_ignore_ascii_case(prefix),
        None => false,
    }
}

/// Estrae un nome tabella indicativo da una SQL semplice `SELECT … FROM x`
/// (serve solo per il nome della `DataTable`, non per l'esecuzione).
pub fn extract_table_name(sql: &str) -> String {
    let upper = sql.to_uppercase();
    if let Some(pos) = upper.find(" FROM ") {
        let rest = sql[pos + 6..].trim_start();
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.' || *c == '"')
            .collect();
        if !name.is_empty() {
            return name.trim_matches('"').to_string();
        }
    }
    "query".to_string()
}
