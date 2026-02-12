use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info};
use nimbus::{app::AppState, app::TabIndex, ui, NimbusConfig, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

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

    if config.providers.aws.is_some() {
        info!("AWS provider configured");
    }
    if config.providers.gcp.is_some() {
        info!("GCP provider configured");
    }
    if config.providers.azure.is_some() {
        info!("Azure provider configured");
    }

    run_tui().await?;

    Ok(())
}

async fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::new();
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
        terminal.draw(|f| ui::render(f, app_state))?;

        if app_state.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app_state.quit(),
                        KeyCode::Tab => app_state.next_tab(),
                        KeyCode::BackTab => app_state.prev_tab(),
                        KeyCode::Char('1') => app_state.set_tab(TabIndex::AWS),
                        KeyCode::Char('2') => app_state.set_tab(TabIndex::GCP),
                        KeyCode::Char('3') => app_state.set_tab(TabIndex::Azure),
                        KeyCode::Char('4') => app_state.set_tab(TabIndex::AllClouds),
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}