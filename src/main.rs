use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info};
use nimbus::{
    app::{AppState, TabIndex},
    core::CloudProvider,
    providers::AWSProvider,
    ui, NimbusConfig, Result,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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

    run_tui(providers).await?;

    Ok(())
}

async fn run_tui(
    providers: Vec<Arc<RwLock<Box<dyn nimbus::core::CloudProvider>>>>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::new().with_providers(providers);

    info!("Loading initial resources...");
    if let Err(e) = app_state.refresh_resources().await {
        error!("Failed to load initial resources: {}", e);
    }

    let result = run_app(&mut terminal, &mut app_state).await;

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

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: &mut AppState,
) -> Result<()> {
    loop {
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
                        match key.code {
                            KeyCode::Char('q') => app_state.quit(),
                            KeyCode::Tab => app_state.next_tab(),
                            KeyCode::BackTab => app_state.prev_tab(),
                            KeyCode::Char('1') => app_state.set_tab(TabIndex::AWS),
                            KeyCode::Char('2') => app_state.set_tab(TabIndex::GCP),
                            KeyCode::Char('3') => app_state.set_tab(TabIndex::Azure),
                            KeyCode::Char('4') => app_state.set_tab(TabIndex::AllClouds),
                            KeyCode::Char('/') => {
                                app_state.enter_filter_mode();
                            }
                            KeyCode::Esc => {
                                if !app_state.filter_text.is_empty() {
                                    app_state.clear_filter();
                                }
                            }
                            KeyCode::Char('r') => {
                                info!("Refreshing resources...");
                                if let Err(e) = app_state.refresh_resources().await {
                                    error!("Refresh failed: {}", e);
                                } else {
                                    info!("Resources refreshed successfully");
                                }
                            }
                            KeyCode::Up => app_state.prev_resource(),
                            KeyCode::Down => app_state.next_resource(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(())
}