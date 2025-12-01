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

    pub async fn add_sticky_window(&self, window_id: u64) -> Result<bool> {
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&window_id) {
            return Err(anyhow::anyhow!("Window not found in Niri"));
        }

        let mut sticky = self.sticky_windows.lock().await;
        Ok(sticky.insert(window_id))
    }

    pub async fn remove_sticky_window(&self, window_id: u64) -> Result<bool> {
        let full_window_list = crate::system_integration::get_full_window_list().await?;
        if !full_window_list.contains(&window_id) {
            return Err(anyhow::anyhow!("Window not found in Niri"));
        }

        let mut sticky = self.sticky_windows.lock().await;
        Ok(sticky.remove(&window_id))
    }

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

    pub async fn stage_window(&self, window_id: u64) -> Result<()> {
        let is_sticky = self.sticky_windows.lock().await.contains(&window_id);
        if !is_sticky {
            return Err(anyhow::anyhow!("Window is not sticky, cannot stage"));
        }

        crate::system_integration::move_to_named_workspace(window_id, "stage").await?;

        let mut sticky = self.sticky_windows.lock().await;
        let mut staged = self.staged_set.lock().await;
        sticky.remove(&window_id);
        staged.insert(window_id);

        Ok(())
    }

    pub async fn stage_active_window(&self) -> Result<()> {
        let id = crate::system_integration::get_active_window_id().await?;
        
        let is_sticky = self.sticky_windows.lock().await.contains(&id);
        if !is_sticky {
            return Err(anyhow::anyhow!("Window is not sticky, cannot stage"));
        }

        crate::system_integration::move_to_named_workspace(id, "stage").await?;

        let mut sticky = self.sticky_windows.lock().await;
        let mut staged = self.staged_set.lock().await;
        sticky.remove(&id);
        staged.insert(id);

        Ok(())
    }

    pub async fn stage_all_windows(&self) -> Result<usize> {
        let sticky_ids = self.sticky_windows.lock().await.clone();
        if sticky_ids.is_empty() {
            return Ok(0);
        }

        let mut staged_count = 0;
        let mut successfully_staged = HashSet::new();

        for id in sticky_ids {
            if crate::system_integration::move_to_named_workspace(id, "stage").await.is_ok() {
                successfully_staged.insert(id);
                staged_count += 1;
            } else {
                eprintln!("Failed to move window {} to stage", id);
            }
        }

        if staged_count > 0 {
            let mut sticky = self.sticky_windows.lock().await;
            let mut staged = self.staged_set.lock().await;
            for id in &successfully_staged {
                sticky.remove(id);
                staged.insert(*id);
            }
        }

        Ok(staged_count)
    }

    pub async fn list_staged_windows(&self) -> Result<Vec<u64>> {
        let staged = self.staged_set.lock().await;
        Ok(staged.iter().copied().collect())
    }

    pub async fn unstage_window(&self, window_id: u64, workspace_id: u64) -> Result<()> {
        let mut staged = self.staged_set.lock().await;
        if !staged.contains(&window_id) {
            return Err(anyhow::anyhow!("Window is not staged"));
        }

        crate::system_integration::move_to_workspace(window_id, workspace_id).await?;

        staged.remove(&window_id);
        let mut sticky = self.sticky_windows.lock().await;
        sticky.insert(window_id);

        Ok(())
    }

    pub async fn unstage_active_window(&self, workspace_id: u64) -> Result<()> {
        let id = crate::system_integration::get_active_window_id().await?;
        let mut staged = self.staged_set.lock().await;
        if !staged.contains(&id) {
            return Err(anyhow::anyhow!("Active window is not staged"));
        }

        crate::system_integration::move_to_workspace(id, workspace_id).await?;

        staged.remove(&id);
        let mut sticky = self.sticky_windows.lock().await;
        sticky.insert(id);

        Ok(())
    }

    pub async fn unstage_all_windows(&self, workspace_id: u64) -> Result<usize> {
        let ids_to_unstage: Vec<u64> = {
            let staged = self.staged_set.lock().await;
            if staged.is_empty() {
                return Ok(0);
            }
            staged.iter().copied().collect()
        };

        let mut successfully_unstaged = HashSet::new();
        for id in &ids_to_unstage {
            if crate::system_integration::move_to_workspace(*id, workspace_id).await.is_ok() {
                successfully_unstaged.insert(*id);
            } else {
                eprintln!("Failed to move window {} to workspace {}", id, workspace_id);
            }
        }

        if !successfully_unstaged.is_empty() {
            let mut staged = self.staged_set.lock().await;
            let mut sticky = self.sticky_windows.lock().await;
            for id in &successfully_unstaged {
                staged.remove(id);
                sticky.insert(*id);
            }
        }

        Ok(successfully_unstaged.len())
    }

    pub async fn handle_workspace_activation(&self, ws_id: u64) -> Result<()> {
        // 更新粘性窗口列表，移除不再存在的窗口
        let sticky_snapshot = {
            let mut sticky = self.sticky_windows.lock().await;
            let full_window_list = crate::system_integration::get_full_window_list().await.unwrap_or_default();
            sticky.retain(|win_id| full_window_list.contains(win_id));
            println!("Updated sticky windows: {:?}", *sticky);
            sticky.clone()
        };

        // 将粘性窗口移动到新工作区
        for win_id in sticky_snapshot.iter() {
            if let Err(_e) = crate::system_integration::move_to_workspace(*win_id, ws_id).await {
                eprintln!("Failed to move window {}: {:?}", win_id, _e);
            }
        }

        Ok(())
    }
}