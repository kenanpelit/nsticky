use anyhow::Result;
use std::collections::HashSet;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct BusinessLogic {
    sticky_windows: std::sync::Arc<Mutex<HashSet<u64>>>,
    staged_set: std::sync::Arc<Mutex<HashSet<u64>>>,
}

impl BusinessLogic {
    pub fn new(
        sticky_windows: std::sync::Arc<Mutex<HashSet<u64>>>,
        staged_set: std::sync::Arc<Mutex<HashSet<u64>>>,
    ) -> Self {
        Self {
            sticky_windows,
            staged_set,
        }
    }

    /// Add window to sticky list
    pub async fn add_sticky_window(&self, window_id: u64) -> Result<bool> {
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&window_id) {
            return Err(anyhow::anyhow!("Window not found in Niri"));
        }

        let mut sticky = self.sticky_windows.lock().await;
        Ok(sticky.insert(window_id))
    }

    /// Remove window from sticky list
    pub async fn remove_sticky_window(&self, window_id: u64) -> Result<bool> {
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&window_id) {
            return Err(anyhow::anyhow!("Window not found in Niri"));
        }

        let mut sticky = self.sticky_windows.lock().await;
        Ok(sticky.remove(&window_id))
    }

    /// List all sticky windows
    pub async fn list_sticky_windows(&self) -> Result<Vec<u64>> {
        let snapshot: Vec<u64> = {
            let sticky = self.sticky_windows.lock().await;
            sticky.iter().copied().collect()
        };
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        let valid_snapshot: Vec<u64> = snapshot
            .into_iter()
            .filter(|id| full_window_list.contains(id))
            .collect();
        Ok(valid_snapshot)
    }

    /// Toggle active window sticky status
    /// Cases: active window in sticky -> remove from sticky, active window not in sticky -> add to sticky
    pub async fn toggle_active_window(&self) -> Result<bool> {
        let active_id = crate::system_integration::get_active_window_id().await?;
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&active_id) {
            return Err(anyhow::anyhow!("Active window not found in Niri"));
        }

        let mut sticky = self.sticky_windows.lock().await;
        if sticky.contains(&active_id) {
            sticky.remove(&active_id);
            Ok(false) // Removed from sticky
        } else {
            sticky.insert(active_id);
            Ok(true) // Added to sticky
        }
    }

    /// Toggle window sticky status by app ID
    /// Cases: window in staged -> move to sticky, window in sticky -> remove from sticky, window in neither -> add to sticky
    pub async fn toggle_by_appid(&self, appid: &str) -> Result<bool> {
        let window_id = crate::system_integration::find_window_by_appid(appid).await?;
        match window_id {
            Some(id) => {
                let full_window_list = crate::system_integration::get_full_window_list().await?;
                if !full_window_list.contains(&id) {
                    return Err(anyhow::anyhow!(
                        "Window with appid {} not found in Niri",
                        appid
                    ));
                }

                let sticky = self.sticky_windows.lock().await;
                let staged = self.staged_set.lock().await;

                if staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    let current_ws_id =
                        crate::system_integration::get_active_workspace_id().await?;
                    crate::system_integration::move_to_workspace(id, current_ws_id).await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    let mut staged = self.staged_set.lock().await;
                    staged.remove(&id);
                    sticky.insert(id);
                    Ok(true)
                } else if sticky.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    let mut sticky = self.sticky_windows.lock().await;
                    sticky.remove(&id);
                    Ok(false)
                } else {
                    drop(sticky);
                    drop(staged);
                    let current_ws_id =
                        crate::system_integration::get_active_workspace_id().await?;
                    crate::system_integration::move_to_workspace(id, current_ws_id).await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    sticky.insert(id);
                    Ok(true)
                }
            }
            None => Err(anyhow::anyhow!("No window found with appid {}", appid)),
        }
    }

    /// Toggle window sticky status by title
    /// Cases: window in staged -> move to sticky, window in sticky -> remove from sticky, window in neither -> add to sticky
    pub async fn toggle_by_title(&self, title: &str) -> Result<bool> {
        let window_id = crate::system_integration::find_window_by_title(title).await?;
        match window_id {
            Some(id) => {
                let full_window_list = crate::system_integration::get_full_window_list().await?;
                if !full_window_list.contains(&id) {
                    return Err(anyhow::anyhow!(
                        "Window with title containing '{}' not found in Niri",
                        title
                    ));
                }

                let sticky = self.sticky_windows.lock().await;
                let staged = self.staged_set.lock().await;

                if staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    let current_ws_id =
                        crate::system_integration::get_active_workspace_id().await?;
                    crate::system_integration::move_to_workspace(id, current_ws_id).await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    let mut staged = self.staged_set.lock().await;
                    staged.remove(&id);
                    sticky.insert(id);
                    Ok(true)
                } else if sticky.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    let mut sticky = self.sticky_windows.lock().await;
                    sticky.remove(&id);
                    Ok(false)
                } else {
                    drop(sticky);
                    drop(staged);
                    let current_ws_id =
                        crate::system_integration::get_active_workspace_id().await?;
                    crate::system_integration::move_to_workspace(id, current_ws_id).await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    sticky.insert(id);
                    Ok(true)
                }
            }
            None => Err(anyhow::anyhow!(
                "No window found with title containing '{}'",
                title
            )),
        }
    }

    /// Toggle window stage status by app ID
    /// Cases: window not in sticky -> error, window in sticky but not staged -> move to staged, window in staged -> move to sticky
    pub async fn toggle_stage_by_appid(&self, appid: &str, workspace_id: u64) -> Result<()> {
        let window_id = crate::system_integration::find_window_by_appid(appid).await?;
        match window_id {
            Some(id) => {
                let full_window_list = crate::system_integration::get_full_window_list().await?;
                if !full_window_list.contains(&id) {
                    return Err(anyhow::anyhow!(
                        "Window with appid {} not found in Niri",
                        appid
                    ));
                }

                let sticky = self.sticky_windows.lock().await;
                let staged = self.staged_set.lock().await;

                if !sticky.contains(&id) && !staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    Err(anyhow::anyhow!(
                        "Window with appid {} is not in sticky list",
                        appid
                    ))
                } else if sticky.contains(&id) && !staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    crate::system_integration::move_to_named_workspace(id, "stage").await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    let mut staged = self.staged_set.lock().await;
                    sticky.remove(&id);
                    staged.insert(id);
                    Ok(())
                } else if !sticky.contains(&id) && staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    crate::system_integration::move_to_workspace(id, workspace_id).await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    let mut staged = self.staged_set.lock().await;
                    staged.remove(&id);
                    sticky.insert(id);
                    Ok(())
                } else {
                    drop(sticky);
                    drop(staged);
                    Err(anyhow::anyhow!(
                        "Unexpected window state for appid {}",
                        appid
                    ))
                }
            }
            None => Err(anyhow::anyhow!("No window found with appid {}", appid)),
        }
    }

    /// Toggle window stage status by title
    /// Cases: window not in sticky -> error, window in sticky but not staged -> move to staged, window in staged -> move to sticky
    pub async fn toggle_stage_by_title(&self, title: &str, workspace_id: u64) -> Result<()> {
        let window_id = crate::system_integration::find_window_by_title(title).await?;
        match window_id {
            Some(id) => {
                let full_window_list = crate::system_integration::get_full_window_list().await?;
                if !full_window_list.contains(&id) {
                    return Err(anyhow::anyhow!(
                        "Window with title containing '{}' not found in Niri",
                        title
                    ));
                }

                let sticky = self.sticky_windows.lock().await;
                let staged = self.staged_set.lock().await;

                if !sticky.contains(&id) && !staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    Err(anyhow::anyhow!(
                        "Window with title containing '{}' is not in sticky list",
                        title
                    ))
                } else if sticky.contains(&id) && !staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    crate::system_integration::move_to_named_workspace(id, "stage").await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    let mut staged = self.staged_set.lock().await;
                    sticky.remove(&id);
                    staged.insert(id);
                    Ok(())
                } else if !sticky.contains(&id) && staged.contains(&id) {
                    drop(sticky);
                    drop(staged);
                    crate::system_integration::move_to_workspace(id, workspace_id).await?;
                    let mut sticky = self.sticky_windows.lock().await;
                    let mut staged = self.staged_set.lock().await;
                    staged.remove(&id);
                    sticky.insert(id);
                    Ok(())
                } else {
                    drop(sticky);
                    drop(staged);
                    Err(anyhow::anyhow!(
                        "Unexpected window state for title containing '{}'",
                        title
                    ))
                }
            }
            None => Err(anyhow::anyhow!(
                "No window found with title containing '{}'",
                title
            )),
        }
    }

    /// Move a sticky window to the stage workspace
    /// Cases: window not in sticky -> error, window already staged -> error, window in sticky -> move to stage
    pub async fn stage_window(&self, window_id: u64) -> Result<()> {
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&window_id) {
            return Err(anyhow::anyhow!("Window not found in Niri"));
        }

        let sticky = self.sticky_windows.lock().await;
        let staged = self.staged_set.lock().await;

        if staged.contains(&window_id) {
            drop(sticky);
            drop(staged);
            return Err(anyhow::anyhow!("Window is already in staged list"));
        }

        let was_sticky = sticky.contains(&window_id);
        if was_sticky {
            drop(sticky);
            drop(staged);
            if let Err(e) =
                crate::system_integration::move_to_named_workspace(window_id, "stage").await
            {
                let mut sticky = self.sticky_windows.lock().await;
                sticky.insert(window_id);
                return Err(e);
            }

            let mut sticky = self.sticky_windows.lock().await;
            let mut staged = self.staged_set.lock().await;
            sticky.remove(&window_id);
            staged.insert(window_id);
            Ok(())
        } else {
            drop(sticky);
            drop(staged);
            Err(anyhow::anyhow!(
                "Window is not in sticky list, cannot stage"
            ))
        }
    }

    /// Move the active sticky window to the stage workspace
    /// Cases: window not in sticky -> error, window already staged -> error, window in sticky -> move to stage
    pub async fn stage_active_window(&self) -> Result<()> {
        let id = crate::system_integration::get_active_window_id().await?;

        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&id) {
            return Err(anyhow::anyhow!("Active window not found in Niri"));
        }

        let sticky = self.sticky_windows.lock().await;
        let staged = self.staged_set.lock().await;

        if staged.contains(&id) {
            drop(sticky);
            drop(staged);
            return Err(anyhow::anyhow!("Window is already in staged list"));
        }

        let was_sticky = sticky.contains(&id);
        if was_sticky {
            drop(sticky);
            drop(staged);
            if let Err(e) = crate::system_integration::move_to_named_workspace(id, "stage").await {
                let mut sticky = self.sticky_windows.lock().await;
                sticky.insert(id);
                return Err(e);
            }

            let mut sticky = self.sticky_windows.lock().await;
            let mut staged = self.staged_set.lock().await;
            sticky.remove(&id);
            staged.insert(id);
            Ok(())
        } else {
            drop(sticky);
            drop(staged);
            Err(anyhow::anyhow!(
                "Window is not in sticky list, cannot stage"
            ))
        }
    }

    /// Check if window is staged
    pub async fn is_window_staged(&self, window_id: u64) -> bool {
        let staged = self.staged_set.lock().await;
        staged.contains(&window_id)
    }

    /// Check if window is sticky
    pub async fn is_window_sticky(&self, window_id: u64) -> bool {
        let sticky = self.sticky_windows.lock().await;
        sticky.contains(&window_id)
    }

    /// Stage all sticky windows
    pub async fn stage_all_windows(&self) -> Result<usize> {
        let sticky_ids = self.sticky_windows.lock().await.clone();
        if sticky_ids.is_empty() {
            return Ok(0);
        }

        let mut successfully_staged = Vec::new();

        let full_window_list = crate::system_integration::get_full_window_list().await?;
        let valid_sticky_ids: Vec<u64> = sticky_ids
            .into_iter()
            .filter(|id| full_window_list.contains(id))
            .collect();

        for id in valid_sticky_ids {
            if crate::system_integration::move_to_named_workspace(id, "stage")
                .await
                .is_ok()
            {
                successfully_staged.push(id);
            } else {
                eprintln!("Failed to move window {} to stage", id);
            }
        }

        let mut sticky = self.sticky_windows.lock().await;
        let mut staged = self.staged_set.lock().await;
        for id in &successfully_staged {
            sticky.remove(id);
            staged.insert(*id);
        }

        Ok(successfully_staged.len())
    }

    /// List all staged windows
    pub async fn list_staged_windows(&self) -> Result<Vec<u64>> {
        let staged = self.staged_set.lock().await;
        Ok(staged.iter().copied().collect())
    }

    /// Move a staged window back to sticky and current workspace
    /// Cases: window already sticky -> error, window not staged -> error, window staged -> move to sticky
    pub async fn unstage_window(&self, window_id: u64, workspace_id: u64) -> Result<()> {
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&window_id) {
            return Err(anyhow::anyhow!("Window not found in Niri"));
        }

        let sticky = self.sticky_windows.lock().await;
        let staged = self.staged_set.lock().await;

        if sticky.contains(&window_id) {
            drop(sticky);
            drop(staged);
            return Err(anyhow::anyhow!("Window is already in sticky list"));
        }

        let was_staged = staged.contains(&window_id);
        if was_staged {
            drop(sticky);
            drop(staged);
            if let Err(e) =
                crate::system_integration::move_to_workspace(window_id, workspace_id).await
            {
                let mut staged = self.staged_set.lock().await;
                staged.insert(window_id);
                return Err(e);
            }

            let mut staged = self.staged_set.lock().await;
            let mut sticky = self.sticky_windows.lock().await;
            staged.remove(&window_id);
            sticky.insert(window_id);

            Ok(())
        } else {
            drop(sticky);
            drop(staged);
            Err(anyhow::anyhow!(
                "Window is not in staged list, cannot unstage"
            ))
        }
    }

    /// Move the active staged window back to sticky and current workspace
    /// Cases: window already sticky -> error, window not staged -> error, window staged -> move to sticky
    pub async fn unstage_active_window(&self, workspace_id: u64) -> Result<()> {
        let id = crate::system_integration::get_active_window_id().await?;

        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&id) {
            return Err(anyhow::anyhow!("Active window not found in Niri"));
        }

        let sticky = self.sticky_windows.lock().await;
        let staged = self.staged_set.lock().await;

        if sticky.contains(&id) {
            drop(sticky);
            drop(staged);
            return Err(anyhow::anyhow!("Window is already in sticky list"));
        }

        let was_staged = staged.contains(&id);
        if was_staged {
            drop(sticky);
            drop(staged);
            if let Err(e) = crate::system_integration::move_to_workspace(id, workspace_id).await {
                let mut staged = self.staged_set.lock().await;
                staged.insert(id);
                return Err(e);
            }

            let mut staged = self.staged_set.lock().await;
            let mut sticky = self.sticky_windows.lock().await;
            staged.remove(&id);
            sticky.insert(id);

            Ok(())
        } else {
            drop(sticky);
            drop(staged);
            Err(anyhow::anyhow!(
                "Window is not in staged list, cannot unstage"
            ))
        }
    }

    /// Unstage all staged windows
    pub async fn unstage_all_windows(&self, workspace_id: u64) -> Result<usize> {
        let ids_to_unstage: Vec<u64> = {
            let staged = self.staged_set.lock().await;
            if staged.is_empty() {
                return Ok(0);
            }
            staged.iter().copied().collect()
        };

        let full_window_list = crate::system_integration::get_full_window_list().await?;
        let valid_ids_to_unstage: Vec<u64> = ids_to_unstage
            .into_iter()
            .filter(|id| full_window_list.contains(id))
            .collect();

        let mut successfully_unstaged = Vec::new();
        for id in &valid_ids_to_unstage {
            if crate::system_integration::move_to_workspace(*id, workspace_id)
                .await
                .is_ok()
            {
                successfully_unstaged.push(*id);
            } else {
                eprintln!("Failed to move window {} to workspace {}", id, workspace_id);
            }
        }

        let mut staged = self.staged_set.lock().await;
        let mut sticky = self.sticky_windows.lock().await;
        for id in &successfully_unstaged {
            staged.remove(id);
            sticky.insert(*id);
        }

        Ok(successfully_unstaged.len())
    }

    /// Handle workspace activation by moving sticky windows to new workspace
    pub async fn handle_workspace_activation(&self, ws_id: u64) -> Result<()> {
        // Update sticky window list, removing non-existent windows
        let sticky_snapshot = {
            let mut sticky = self.sticky_windows.lock().await;
            let full_window_list = crate::system_integration::get_full_window_list()
                .await
                .unwrap_or_default();
            sticky.retain(|win_id| full_window_list.contains(win_id));
            println!("Updated sticky windows: {:?}", *sticky);
            sticky.clone()
        };

        // Move sticky windows to new workspace
        for win_id in sticky_snapshot.iter() {
            if let Err(_e) = crate::system_integration::move_to_workspace(*win_id, ws_id).await {
                eprintln!("Failed to move window {}: {:?}", win_id, _e);
            }
        }

        Ok(())
    }
}
