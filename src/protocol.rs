use anyhow::Result;

/// Define request types
#[derive(Debug)]
pub enum Request {
    Add { window_id: u64 },
    Remove { window_id: u64 },
    List,
    ToggleActive,
    ToggleAppid { appid: String },
    ToggleTitle { title: String },
    Stage(StageArgs),
    Unstage(UnstageArgs),
}

#[derive(Debug, Default)]
pub struct StageArgs {
    pub window_id: Option<u64>,
    pub all: bool,
    pub list: bool,
    pub active: bool,
    pub appid: Option<String>,
    pub title: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct UnstageArgs {
    pub window_id: Option<u64>,
    pub all: bool,
    pub active: bool,
    pub appid: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug)]
pub enum Response {
    Success(String),
    Error(String),
    Data(String),
}

/// Parse string command to Request
pub fn parse_request(line: &str) -> Result<Request> {
    let line = line.trim();
    let mut parts = line.split_whitespace();

    match parts.next() {
        Some("add") => {
            if let Some(id_str) = parts.next() {
                if let Ok(id) = id_str.parse::<u64>() {
                    Ok(Request::Add { window_id: id })
                } else {
                    Err(anyhow::anyhow!("Invalid window id"))
                }
            } else {
                Err(anyhow::anyhow!("Missing window id"))
            }
        }
        Some("remove") => {
            if let Some(id_str) = parts.next() {
                if let Ok(id) = id_str.parse::<u64>() {
                    Ok(Request::Remove { window_id: id })
                } else {
                    Err(anyhow::anyhow!("Invalid window id"))
                }
            } else {
                Err(anyhow::anyhow!("Missing window id"))
            }
        }
        Some("list") => Ok(Request::List),
        Some("toggle_active") => Ok(Request::ToggleActive),
        Some("toggle_appid") => {
            if let Some(appid) = parts.next() {
                Ok(Request::ToggleAppid {
                    appid: appid.to_string(),
                })
            } else {
                Err(anyhow::anyhow!("Missing appid"))
            }
        }
        Some("toggle_title") => {
            // 标题可能包含空格，所以我们将剩余部分连接起来
            let title = parts.collect::<Vec<_>>().join(" ");
            if title.is_empty() {
                Err(anyhow::anyhow!("Missing title"))
            } else {
                Ok(Request::ToggleTitle { title })
            }
        }
        Some("stage") => {
            let arg = parts.next();
            if arg == Some("--toggle-appid") {
                if let Some(appid) = parts.next() {
                    let stage_args = StageArgs {
                        appid: Some(appid.to_string()),
                        ..Default::default()
                    };
                    return Ok(Request::Stage(stage_args));
                } else {
                    return Err(anyhow::anyhow!("Missing appid for toggle"));
                }
            } else if arg == Some("--toggle-title") {
                // Title may contain spaces, join remaining parts
                let title = parts.collect::<Vec<_>>().join(" ");
                if title.is_empty() {
                    return Err(anyhow::anyhow!("Missing title for toggle"));
                } else {
                    let stage_args = StageArgs {
                        title: Some(title),
                        ..Default::default()
                    };
                    return Ok(Request::Stage(stage_args));
                }
            }

            match arg {
                Some("--all") => Ok(Request::Stage(StageArgs {
                    window_id: None,
                    all: true,
                    list: false,
                    active: false,
                    appid: None,
                    title: None,
                })),
                Some("--list") => Ok(Request::Stage(StageArgs {
                    window_id: None,
                    all: false,
                    list: true,
                    active: false,
                    appid: None,
                    title: None,
                })),
                Some("--active") => Ok(Request::Stage(StageArgs {
                    window_id: None,
                    all: false,
                    list: false,
                    active: true,
                    appid: None,
                    title: None,
                })),
                Some("--appid") => {
                    if let Some(appid) = parts.next() {
                        Ok(Request::Stage(StageArgs {
                            window_id: None,
                            all: false,
                            list: false,
                            active: false,
                            appid: Some(appid.to_string()),
                            title: None,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Missing appid for stage"))
                    }
                }
                Some("--title") => {
                    // Title may contain spaces, join remaining parts
                    let title = parts.collect::<Vec<_>>().join(" ");
                    if title.is_empty() {
                        Err(anyhow::anyhow!("Missing title for stage"))
                    } else {
                        Ok(Request::Stage(StageArgs {
                            window_id: None,
                            all: false,
                            list: false,
                            active: false,
                            appid: None,
                            title: Some(title),
                        }))
                    }
                }
                Some(id_str) => {
                    if let Ok(id) = id_str.parse::<u64>() {
                        Ok(Request::Stage(StageArgs {
                            window_id: Some(id),
                            all: false,
                            list: false,
                            active: false,
                            appid: None,
                            title: None,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Invalid window id"))
                    }
                }
                None => Err(anyhow::anyhow!("Missing argument for stage")),
            }
        }
        Some("unstage") => {
            let arg = parts.next();
            if arg == Some("--toggle-appid") {
                if let Some(appid) = parts.next() {
                    return Ok(Request::ToggleAppid {
                        appid: appid.to_string(),
                    });
                } else {
                    return Err(anyhow::anyhow!("Missing appid for toggle"));
                }
            } else if arg == Some("--toggle-title") {
                // Title may contain spaces, join remaining parts
                let title = parts.collect::<Vec<_>>().join(" ");
                if title.is_empty() {
                    return Err(anyhow::anyhow!("Missing title for toggle"));
                } else {
                    return Ok(Request::ToggleTitle { title });
                }
            }

            match arg {
                Some("--all") => Ok(Request::Unstage(UnstageArgs {
                    window_id: None,
                    all: true,
                    active: false,
                    appid: None,
                    title: None,
                })),
                Some("--active") => Ok(Request::Unstage(UnstageArgs {
                    window_id: None,
                    all: false,
                    active: true,
                    appid: None,
                    title: None,
                })),
                Some("--appid") => {
                    if let Some(appid) = parts.next() {
                        Ok(Request::Unstage(UnstageArgs {
                            window_id: None,
                            all: false,
                            active: false,
                            appid: Some(appid.to_string()),
                            title: None,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Missing appid for unstage"))
                    }
                }
                Some("--title") => {
                    // Title may contain spaces, join remaining parts
                    let title = parts.collect::<Vec<_>>().join(" ");
                    if title.is_empty() {
                        Err(anyhow::anyhow!("Missing title for unstage"))
                    } else {
                        Ok(Request::Unstage(UnstageArgs {
                            window_id: None,
                            all: false,
                            active: false,
                            appid: None,
                            title: Some(title),
                        }))
                    }
                }
                Some(id_str) => {
                    if let Ok(id) = id_str.parse::<u64>() {
                        Ok(Request::Unstage(UnstageArgs {
                            window_id: Some(id),
                            all: false,
                            active: false,
                            appid: None,
                            title: None,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Invalid window id"))
                    }
                }
                None => Err(anyhow::anyhow!("Missing argument for unstage")),
            }
        }
        _ => Err(anyhow::anyhow!("Unknown command")),
    }
}

/// Convert Response to string
pub fn format_response(response: Response) -> String {
    match response {
        Response::Success(msg) => msg,
        Response::Error(msg) => format!("Error: {msg}"),
        Response::Data(data) => data,
    }
}
