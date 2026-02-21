use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info, warn};
use nimbus::{
    app::{AppState, TabIndex, ViewMode},
    cache::CacheStore,
    core::CloudProvider,
    providers::AWSProvider,
    ui, NimbusConfig, Result,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

fn setup_logging() -> Result<()> {
    let log_dir = dirs::home_dir()
        .ok_or_else(|| nimbus::NimbusError::ConfigError("Could not determine home directory".to_string()))?
        .join(".nimbus");
    
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file_path = log_dir.join("nimbus.log");
    
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    )
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
                Some(Arc::new(store))
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
                providers.push(Arc::new(RwLock::new(Box::new(aws_provider)
                    as Box<dyn nimbus::core::CloudProvider>)));
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
    cache_store: Option<Arc<CacheStore>>,
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
    
    let mut loaded_from_cache = false;
    if let Some(ref cache) = cache_store {
        info!("Attempting to load resources from cache...");
        match load_resources_from_cache(&mut app_state, cache).await {
            Ok(count) if count > 0 => {
                info!("Loaded {} resources from cache", count);
                loaded_from_cache = true;
                app_state.set_success(format!("Loaded {} resources from cache (press 'r' to refresh)", count));
            }
            Ok(_) => {
                info!("No cached resources found");
            }
            Err(e) => {
                warn!("Failed to load from cache: {}", e);
            }
        }
    }

    if !loaded_from_cache {
        info!("Fetching fresh resources from providers...");
        if let Err(e) = refresh_and_cache_resources(&mut app_state, &cache_store).await {
            error!("Failed to load initial resources: {}", e);
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

async fn load_resources_from_cache(
    app_state: &mut AppState,
    cache: &CacheStore,
) -> Result<usize> {
    let cached_resources = cache.get_all_cached_resources()?;
    
    if cached_resources.is_empty() {
        return Ok(0);
    }

    let count = cached_resources.len();
    
    if let Some(first) = cached_resources.first() {
        app_state.last_refresh = Some(first.cached_at);
    }

    Ok(count)
}

async fn refresh_and_cache_resources(
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Result<()> {
    app_state.refresh_resources().await?;
    
    if let Some(ref cache) = cache_store {
        let resources = app_state.resources.read().await;
        info!("Caching {} resources", resources.len());
        
        match cache.cache_resources(&resources) {
            Ok(_) => {
                info!("Resources cached successfully");
            }
            Err(e) => {
                warn!("Failed to cache resources: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: &mut AppState,
    cache_store: Option<Arc<CacheStore>>,
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
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(future)
            });
        })?;

        if app_state.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app_state.show_confirmation {
                        match key.code {
                            KeyCode::Enter => {
                                app_state.cancel_confirmation();
                                
                                let action_info = if let Some(resource_idx) = app_state.get_selected_resource_index() {
                                    let resources = app_state.resources.read().await;
                                    if let Some(resource) = resources.get(resource_idx) {
                                        let actions = resource.supported_actions();
                                        if let Some(action) = actions.get(app_state.selected_action) {
                                            Some((
                                                resource.id().to_string(),
                                                resource.name().to_string(),
                                                resource.provider(),
                                                *action
                                            ))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                
                                if let Some((resource_id, resource_name, resource_provider, action)) = action_info {
                                    info!("Executing action {:?} on resource {}", action, resource_id);
                                    app_state.start_loading();
                                    
                                    let mut action_result = None;
                                    for provider in &app_state.providers {
                                        let provider = provider.read().await;
                                        if provider.provider_type() == resource_provider {
                                            action_result = Some(provider.execute_action(&resource_id, action).await);
                                            break;
                                        }
                                    }
                                    
                                    match action_result {
                                        Some(Ok(_)) => {
                                            info!("Action executed successfully");
                                            let success_msg = format!(
                                                "Successfully {} '{}'",
                                                match action {
                                                    nimbus::core::Action::Start => "started",
                                                    nimbus::core::Action::Stop => "stopped",
                                                    nimbus::core::Action::Restart => "restarted",
                                                    nimbus::core::Action::Terminate => "terminated",
                                                    _ => "completed action on",
                                                },
                                                resource_name
                                            );
                                            app_state.record_action(success_msg.clone());
                                            app_state.set_success(success_msg);
                                            last_message_time = Some(std::time::Instant::now());
                                            
                                            if let Err(e) = refresh_and_cache_resources(app_state, &cache_store).await {
                                                error!("Failed to refresh after action: {}", e);
                                            }
                                        }
                                        Some(Err(e)) => {
                                            error!("Action failed: {}", e);
                                            app_state.set_error(format!("{}", e));
                                        }
                                        None => {
                                            error!("No provider found for resource");
                                            app_state.set_error("No provider found for this resource".to_string());
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                app_state.cancel_confirmation();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if app_state.is_filtering() {
                        match key.code {
                            KeyCode::Char(c) => {
                                app_state.push_filter_char(c);
                            }
                            KeyCode::Backspace => {
                                app_state.pop_filter_char();
                            }
                            KeyCode::Esc => {
                                app_state.exit_filter_mode();
                                if app_state.filter_text.is_empty() {
                                    app_state.apply_filter();
                                }
                            }
                            KeyCode::Enter => {
                                app_state.exit_filter_mode();
                            }
                            _ => {}
                        }
                    } else {
                        match app_state.view_mode {
                            ViewMode::Dashboard | ViewMode::ResourceList => {
                                match key.code {
                                    KeyCode::Char('q') => app_state.quit(),
                                    KeyCode::Tab => app_state.next_tab(),
                                    KeyCode::BackTab => app_state.prev_tab(),
                                    KeyCode::Char('1') => app_state.set_tab(TabIndex::AWS),
                                    KeyCode::Char('2') => app_state.set_tab(TabIndex::GCP),
                                    KeyCode::Char('3') => app_state.set_tab(TabIndex::Azure),
                                    KeyCode::Char('4') => app_state.set_tab(TabIndex::AllClouds),
                                    KeyCode::Char('d') => {
                                        app_state.toggle_view_mode();
                                        app_state.clear_messages();
                                    }
                                    KeyCode::Char('c') => {
                                        if let Some(ref cache) = cache_store {
                                            info!("Clearing cache...");
                                            match cache.clear_cache(None) {
                                                Ok(_) => {
                                                    let msg = "Cache cleared successfully".to_string();
                                                    app_state.record_action(msg.clone());
                                                    app_state.set_success(msg);
                                                    last_message_time = Some(std::time::Instant::now());
                                                    info!("Cache cleared");
                                                }
                                                Err(e) => {
                                                    error!("Failed to clear cache: {}", e);
                                                    app_state.set_error(format!("Failed to clear cache: {}", e));
                                                }
                                            }
                                        } else {
                                            app_state.set_error("Cache is not enabled".to_string());
                                        }
                                    }
                                    KeyCode::Char('/') => {
                                        if matches!(app_state.view_mode, ViewMode::ResourceList) {
                                            app_state.enter_filter_mode();
                                        }
                                    }
                                    KeyCode::Esc => {
                                        if !app_state.filter_text.is_empty() {
                                            app_state.clear_filter();
                                        } else {
                                            app_state.clear_messages();
                                        }
                                    }
                                    KeyCode::Char('r') => {
                                        info!("Refreshing resources...");
                                        app_state.clear_messages();
                                        if let Err(e) = refresh_and_cache_resources(app_state, &cache_store).await {
                                            error!("Refresh failed: {}", e);
                                        } else {
                                            info!("Resources refreshed successfully");
                                            let msg = "Resources refreshed successfully".to_string();
                                            app_state.record_action(msg.clone());
                                            app_state.set_success(msg);
                                            last_message_time = Some(std::time::Instant::now());
                                        }
                                    }
                                    KeyCode::Up => {
                                        if matches!(app_state.view_mode, ViewMode::ResourceList) {
                                            app_state.prev_resource();
                                        }
                                    }
                                    KeyCode::Down => {
                                        if matches!(app_state.view_mode, ViewMode::ResourceList) {
                                            app_state.next_resource();
                                        }
                                    }
                                    KeyCode::Enter => {
                                        if matches!(app_state.view_mode, ViewMode::ResourceList) {
                                            app_state.clear_messages();
                                            app_state.enter_detail_view();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            ViewMode::ResourceDetail => {
                                match key.code {
                                    KeyCode::Char('q') => app_state.quit(),
                                    KeyCode::Esc => {
                                        app_state.clear_messages();
                                        app_state.exit_detail_view();
                                    }
                                    KeyCode::Up => {
                                        let action_count = {
                                            let resources = app_state.resources.read().await;
                                            if let Some(resource_idx) = app_state.get_selected_resource_index() {
                                                if let Some(resource) = resources.get(resource_idx) {
                                                    resource.supported_actions().len()
                                                } else {
                                                    0
                                                }
                                            } else {
                                                0
                                            }
                                        };
                                        app_state.prev_action(action_count);
                                    }
                                    KeyCode::Down => {
                                        let action_count = {
                                            let resources = app_state.resources.read().await;
                                            if let Some(resource_idx) = app_state.get_selected_resource_index() {
                                                if let Some(resource) = resources.get(resource_idx) {
                                                    resource.supported_actions().len()
                                                } else {
                                                    0
                                                }
                                            } else {
                                                0
                                            }
                                        };
                                        app_state.next_action(action_count);
                                    }
                                    KeyCode::Enter => {
                                        let action_info = {
                                            let resources = app_state.resources.read().await;
                                            if let Some(resource_idx) = app_state.get_selected_resource_index() {
                                                if let Some(resource) = resources.get(resource_idx) {
                                                    let actions = resource.supported_actions();
                                                    if let Some(action) = actions.get(app_state.selected_action) {
                                                        Some((
                                                            resource.id().to_string(),
                                                            resource.name().to_string(),
                                                            resource.provider(),
                                                            *action
                                                        ))
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        };
                                        
                                        if let Some((resource_id, resource_name, resource_provider, action)) = action_info {
                                            if action.is_destructive() {
                                                let message = format!(
                                                    "Are you sure you want to {} '{}'?\n\nThis action cannot be undone.\n\nPress Enter to confirm or ESC to cancel.",
                                                    action.as_str().to_lowercase(),
                                                    resource_name
                                                );
                                                app_state.show_action_confirmation(message);
                                            } else {
                                                info!("Executing non-destructive action {:?}", action);
                                                app_state.start_loading();
                                                
                                                let mut action_result = None;
                                                for provider in &app_state.providers {
                                                    let provider = provider.read().await;
                                                    if provider.provider_type() == resource_provider {
                                                        action_result = Some(provider.execute_action(&resource_id, action).await);
                                                        break;
                                                    }
                                                }
                                                
                                                match action_result {
                                                    Some(Ok(_)) => {
                                                        info!("Action executed successfully");
                                                        let success_msg = format!(
                                                            "Successfully {} '{}'",
                                                            match action {
                                                                nimbus::core::Action::Start => "started",
                                                                nimbus::core::Action::Stop => "stopped",
                                                                nimbus::core::Action::Restart => "restarted",
                                                                _ => "completed action on",
                                                            },
                                                            resource_name
                                                        );
                                                        app_state.record_action(success_msg.clone());
                                                        app_state.set_success(success_msg);
                                                        last_message_time = Some(std::time::Instant::now());
                                                        
                                                        if let Err(e) = refresh_and_cache_resources(app_state, &cache_store).await {
                                                            error!("Failed to refresh: {}", e);
                                                        }
                                                    }
                                                    Some(Err(e)) => {
                                                        error!("Action failed: {}", e);
                                                        app_state.set_error(format!("{}", e));
                                                    }
                                                    None => {
                                                        error!("No provider found for resource");
                                                        app_state.set_error("No provider found for this resource".to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}