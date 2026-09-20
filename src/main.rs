use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info, warn};
use nimbus::{
    app::{handle_key_event, refresh_and_cache_resources, AppState},
    cache::CacheStore,
    core::CloudProvider,
    providers::AWSProvider,
    ui, NimbusConfig, Result,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

fn setup_logging() -> Result<()> {
    let log_dir = dirs::home_dir()
        .ok_or_else(|| {
            nimbus::NimbusError::ConfigError("Could not determine home directory".to_string())
        })?
        .join(".nimbus");

    std::fs::create_dir_all(&log_dir)?;

    let log_file_path = log_dir.join("nimbus.log");

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging()?;

    info!("Nimbus - Cloud Resource Manager");
    info!("Starting application...");

    let config = match NimbusConfig::load() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            info!("Using default configuration");
            NimbusConfig::default()
        }
    };

    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        error!("Please configure at least one cloud provider.");
        error!(
            "Configuration file location: {:?}",
            NimbusConfig::config_file_path()
        );
        return Err(e);
    }

    info!("Configuration validated successfully");

    let cache_store = if config.cache.enabled {
        let db_path = config.cache.get_db_path();
        info!("Initializing cache at: {:?}", db_path);

        match CacheStore::new(&db_path, config.cache.max_age_hours) {
            Ok(store) => {
                info!("Cache initialized successfully");
                Some(Rc::new(store))
            }
            Err(e) => {
                warn!("Failed to initialize cache: {}", e);
                warn!("Continuing without cache");
                None
            }
        }
    } else {
        info!("Cache disabled in configuration");
        None
    };

    let mut providers: Vec<Arc<RwLock<Box<dyn nimbus::core::CloudProvider>>>> = Vec::new();

    if let Some(aws_config) = config.providers.aws {
        info!("Initializing AWS provider...");
        let mut aws_provider = AWSProvider::new(aws_config);

        match aws_provider.authenticate().await {
            Ok(_) => {
                info!("AWS provider authenticated successfully");
                providers.push(Arc::new(RwLock::new(
                    Box::new(aws_provider) as Box<dyn nimbus::core::CloudProvider>
                )));
            }
            Err(e) => {
                error!("AWS authentication failed: {}", e);
                error!("Continuing without AWS provider");
            }
        }
    }

    if config.providers.gcp.is_some() {
        info!("GCP provider configured (not implemented yet)");
    }
    if config.providers.azure.is_some() {
        info!("Azure provider configured (not implemented yet)");
    }

    if providers.is_empty() {
        error!("No providers available. Please check your configuration.");
        return Err(nimbus::NimbusError::ConfigError(
            "No cloud providers available".to_string(),
        ));
    }

    run_tui(providers, cache_store).await?;

    Ok(())
}

async fn run_tui(
    providers: Vec<Arc<RwLock<Box<dyn nimbus::core::CloudProvider>>>>,
    cache_store: Option<Rc<CacheStore>>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let cache_enabled = cache_store.is_some();
    let mut app_state = AppState::new()
        .with_providers(providers)
        .with_cache_enabled(cache_enabled);

    info!("Loading initial resources...");

    // Try to load from cache first if available
    // This provides instant startup if we have cached data
    let loaded_from_cache = if let Some(ref cache) = cache_store {
        info!("Checking cache for existing resources...");
        match cache.get_all_cached_resources() {
            Ok(cached_resources) if !cached_resources.is_empty() => {
                info!("Found {} cached resources", cached_resources.len());

                if let Some(first) = cached_resources.first() {
                    app_state.last_refresh = Some(first.cached_at);
                    let age = chrono::Utc::now().signed_duration_since(first.cached_at);
                    info!("Cache age: {}", format_duration(age));
                }

                // Note: The actual cached resources are in the database
                // We just set the timestamp here for the cache age display
                // The refresh call below will populate the actual resources
                true
            }
            Ok(_) => {
                info!("Cache is empty");
                false
            }
            Err(e) => {
                warn!("Failed to query cache: {}", e);
                false
            }
        }
    } else {
        false
    };

    // ALWAYS fetch fresh resources on startup
    // This ensures the user sees data immediately without needing to press 'r'
    // Even if we have cache, we fetch fresh data to ensure accuracy
    info!("Fetching fresh resources from cloud providers...");
    match refresh_and_cache_resources(&mut app_state, &cache_store).await {
        Ok(_) => {
            if loaded_from_cache {
                info!("Fresh resources loaded and cache updated");
            } else {
                info!("Initial resources loaded successfully");
            }
        }
        Err(e) => {
            error!("Failed to load resources: {}", e);
            // If fresh fetch fails but we had cache, the user can still browse cached data
            // Otherwise they'll see the empty state with an error message
            app_state.set_error(format!("Failed to load resources: {}", e));
        }
    }

    let result = run_app(&mut terminal, &mut app_state, cache_store).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        error!("Application error: {}", e);
        return Err(e);
    }

    info!("Application exited successfully");
    Ok(())
}

fn format_duration(duration: chrono::Duration) -> String {
    if duration.num_minutes() < 1 {
        "less than a minute".to_string()
    } else if duration.num_hours() < 1 {
        format!("{} minutes", duration.num_minutes())
    } else if duration.num_days() < 1 {
        format!("{} hours", duration.num_hours())
    } else {
        format!("{} days", duration.num_days())
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: &mut AppState,
    cache_store: Option<Rc<CacheStore>>,
) -> Result<()> {
    let mut last_message_time: Option<std::time::Instant> = None;
    const MESSAGE_DISPLAY_DURATION: Duration = Duration::from_secs(3);

    loop {
        if let Some(msg_time) = last_message_time {
            if msg_time.elapsed() > MESSAGE_DISPLAY_DURATION {
                app_state.clear_success();
                last_message_time = None;
            }
        }

        terminal.draw(|f| {
            let future = ui::render(f, app_state);
            tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future));
        })?;

        if app_state.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(shown_at) =
                        handle_key_event(key.code, app_state, &cache_store).await
                    {
                        last_message_time = Some(shown_at);
                    }
                }
            }
        }
    }

    Ok(())
}