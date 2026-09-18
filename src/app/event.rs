//! Keyboard event handling for the TUI.
//!
//! Moved out of `main.rs` (which used to hold the whole match tree inline).
//! `refresh_and_cache_resources` lives here too since it's only ever called
//! from event handling (after an action, on manual refresh) and from
//! startup in `main.rs` — keeping one copy avoids the two call sites
//! drifting apart.

use crate::app::{AppState, TabIndex, ViewMode};
use crate::cache::CacheStore;
use crate::core::{Action, Provider, ResourceType};
use crate::error::Result;
use crossterm::event::KeyCode;
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;

/// Refreshes resources from all providers and writes the result to cache
/// (if caching is enabled). Used on startup, on manual refresh ('r'), and
/// after any resource action so the UI reflects the new state.
pub async fn refresh_and_cache_resources(
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Result<()> {
    app_state.refresh_resources().await?;

    if let Some(ref cache) = cache_store {
        let resources = app_state.resources.read().await;
        let resource_count = resources.len();

        info!("Writing {} resources to cache", resource_count);

        match cache.cache_resources(&resources) {
            Ok(_) => {
                info!("Successfully cached {} resources", resource_count);
            }
            Err(e) => {
                warn!("Failed to cache resources: {}", e);
            }
        }
    }

    Ok(())
}

/// Handles a single key press and returns `Some(Instant)` if a status
/// message was just shown (so the caller's loop knows to start the
/// display-duration countdown), or `None` otherwise.
pub async fn handle_key_event(
    key_code: KeyCode,
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Option<Instant> {
    if app_state.show_confirmation {
        return handle_confirmation_mode(key_code, app_state, cache_store).await;
    }

    if app_state.is_filtering() {
        handle_filter_mode(key_code, app_state);
        return None;
    }

    match app_state.view_mode {
        ViewMode::Dashboard | ViewMode::ResourceList => {
            handle_list_mode(key_code, app_state, cache_store).await
        }
        ViewMode::ResourceDetail => handle_detail_mode(key_code, app_state, cache_store).await,
    }
}

async fn handle_confirmation_mode(
    key_code: KeyCode,
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Option<Instant> {
    match key_code {
        KeyCode::Enter => {
            app_state.cancel_confirmation();

            let action_info = selected_resource_action(app_state).await;
            if let Some((resource_id, resource_name, resource_provider, resource_type, action)) =
                action_info
            {
                return execute_action_on_resource(
                    &resource_id,
                    &resource_name,
                    resource_provider,
                    resource_type,
                    action,
                    app_state,
                    cache_store,
                )
                .await;
            }
            None
        }
        KeyCode::Esc => {
            app_state.cancel_confirmation();
            None
        }
        _ => None,
    }
}

fn handle_filter_mode(key_code: KeyCode, app_state: &mut AppState) {
    match key_code {
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
}

async fn handle_list_mode(
    key_code: KeyCode,
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Option<Instant> {
    match key_code {
        KeyCode::Char('q') => {
            app_state.quit();
            None
        }
        KeyCode::Tab => {
            app_state.next_tab();
            None
        }
        KeyCode::BackTab => {
            app_state.prev_tab();
            None
        }
        KeyCode::Char('1') => {
            app_state.set_tab(TabIndex::AWS);
            None
        }
        KeyCode::Char('2') => {
            app_state.set_tab(TabIndex::GCP);
            None
        }
        KeyCode::Char('3') => {
            app_state.set_tab(TabIndex::Azure);
            None
        }
        KeyCode::Char('4') => {
            app_state.set_tab(TabIndex::AllClouds);
            None
        }
        KeyCode::Char('d') => {
            app_state.toggle_view_mode();
            app_state.clear_messages();
            None
        }
        KeyCode::Char('c') => {
            if let Some(ref cache) = cache_store {
                info!("User requested cache clear");
                match cache.clear_cache(None) {
                    Ok(_) => {
                        let msg = "Cache cleared successfully".to_string();
                        app_state.record_action(msg.clone());
                        app_state.set_success(msg);
                        info!("Cache cleared");
                        Some(Instant::now())
                    }
                    Err(e) => {
                        error!("Failed to clear cache: {}", e);
                        app_state.set_error(format!("Failed to clear cache: {}", e));
                        None
                    }
                }
            } else {
                app_state.set_error("Cache is not enabled".to_string());
                None
            }
        }
        KeyCode::Char('/') => {
            if matches!(app_state.view_mode, ViewMode::ResourceList) {
                app_state.enter_filter_mode();
            }
            None
        }
        KeyCode::Esc => {
            if !app_state.filter_text.is_empty() {
                app_state.clear_filter();
            } else {
                app_state.clear_messages();
            }
            None
        }
        KeyCode::Char('r') => {
            info!("User requested manual refresh");
            app_state.clear_messages();
            if let Err(e) = refresh_and_cache_resources(app_state, cache_store).await {
                error!("Refresh failed: {}", e);
                None
            } else {
                info!("Refresh completed successfully");
                let msg = "Resources refreshed successfully".to_string();
                app_state.record_action(msg.clone());
                app_state.set_success(msg);
                Some(Instant::now())
            }
        }
        KeyCode::Up => {
            if matches!(app_state.view_mode, ViewMode::ResourceList) {
                app_state.prev_resource();
            }
            None
        }
        KeyCode::Down => {
            if matches!(app_state.view_mode, ViewMode::ResourceList) {
                app_state.next_resource();
            }
            None
        }
        KeyCode::Enter => {
            if matches!(app_state.view_mode, ViewMode::ResourceList) {
                app_state.clear_messages();
                app_state.enter_detail_view();
            }
            None
        }
        _ => None,
    }
}

async fn handle_detail_mode(
    key_code: KeyCode,
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Option<Instant> {
    match key_code {
        KeyCode::Char('q') => {
            app_state.quit();
            None
        }
        KeyCode::Esc => {
            app_state.clear_messages();
            app_state.exit_detail_view();
            None
        }
        KeyCode::Up => {
            let action_count = selected_resource_action_count(app_state).await;
            app_state.prev_action(action_count);
            None
        }
        KeyCode::Down => {
            let action_count = selected_resource_action_count(app_state).await;
            app_state.next_action(action_count);
            None
        }
        KeyCode::Enter => {
            let action_info = selected_resource_action(app_state).await;
            if let Some((resource_id, resource_name, resource_provider, resource_type, action)) =
                action_info
            {
                if action.is_destructive() {
                    let message = format!(
                        "Are you sure you want to {} '{}'?\n\nThis action cannot be undone.\n\nPress Enter to confirm or ESC to cancel.",
                        action.as_str().to_lowercase(),
                        resource_name
                    );
                    app_state.show_action_confirmation(message);
                    None
                } else {
                    execute_action_on_resource(
                        &resource_id,
                        &resource_name,
                        resource_provider,
                        resource_type,
                        action,
                        app_state,
                        cache_store,
                    )
                    .await
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Looks up the resource under the cursor and the action currently
/// highlighted in its action list, if any.
async fn selected_resource_action(
    app_state: &AppState,
) -> Option<(String, String, Provider, ResourceType, Action)> {
    let resource_idx = app_state.get_selected_resource_index()?;
    let resources = app_state.resources.read().await;
    let resource = resources.get(resource_idx)?;
    let actions = resource.supported_actions();
    let action = actions.get(app_state.selected_action)?;
    Some((
        resource.id().to_string(),
        resource.name().to_string(),
        resource.provider(),
        resource.resource_type(),
        *action,
    ))
}

async fn selected_resource_action_count(app_state: &AppState) -> usize {
    let Some(resource_idx) = app_state.get_selected_resource_index() else {
        return 0;
    };
    let resources = app_state.resources.read().await;
    resources
        .get(resource_idx)
        .map(|r| r.supported_actions().len())
        .unwrap_or(0)
}

/// Runs a resource action against whichever provider owns it, updates
/// app state with the result, and refreshes resources on success.
/// Shared by the confirmation-confirm path and the non-destructive
/// detail-view path (Terminate never reaches the latter, since it's
/// always destructive and routes through confirmation first).
async fn execute_action_on_resource(
    resource_id: &str,
    resource_name: &str,
    resource_provider: Provider,
    resource_type: ResourceType,
    action: Action,
    app_state: &mut AppState,
    cache_store: &Option<Arc<CacheStore>>,
) -> Option<Instant> {
    info!("Executing action {:?} on resource {}", action, resource_id);
    app_state.start_loading();

    let mut action_result = None;
    for provider in &app_state.providers {
        let provider = provider.read().await;
        if provider.provider_type() == resource_provider {
            action_result = Some(
                provider
                    .execute_action(resource_id, resource_type, action)
                    .await,
            );
            break;
        }
    }

    match action_result {
        Some(Ok(_)) => {
            info!("Action executed successfully");
            let success_msg = format!(
                "Successfully {} '{}'",
                match action {
                    Action::Start => "started",
                    Action::Stop => "stopped",
                    Action::Restart => "restarted",
                    Action::Terminate => "terminated",
                    _ => "completed action on",
                },
                resource_name
            );
            app_state.record_action(success_msg.clone());
            app_state.set_success(success_msg);

            if let Err(e) = refresh_and_cache_resources(app_state, cache_store).await {
                error!("Failed to refresh after action: {}", e);
            }

            Some(Instant::now())
        }
        Some(Err(e)) => {
            error!("Action failed: {}", e);
            app_state.set_error(format!("{}", e));
            None
        }
        None => {
            error!("No provider found for resource");
            app_state.set_error("No provider found for this resource".to_string());
            None
        }
    }
}