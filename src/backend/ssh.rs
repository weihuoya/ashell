use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use directories::BaseDirs;
use russh::{
    ChannelMsg, Disconnect,
    client::{self, Handler},
    keys::{HashAlg, PrivateKey, decode_secret_key, key::PrivateKeyWithHashAlg, load_secret_key},
};
use tokio::sync::mpsc;

use crate::{
    session::config::{AuthMethod, Session},
    system::{SystemSnapshot, remote_snapshot_from_kv},
    terminal::{BackendCommand, BackendEvent, BackendTx},
};

pub fn spawn_ssh_terminal(
    runtime: &tokio::runtime::Handle,
    tab_id: String,
    session: Session,
    cols: u16,
    rows: u16,
    events: std::sync::mpsc::Sender<BackendEvent>,
) -> BackendTx {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<BackendCommand>();
    let task_tab = tab_id.clone();
    runtime.spawn(async move {
        if let Err(err) = run_ssh(
            task_tab.clone(),
            session,
            cols,
            rows,
            cmd_rx,
            events.clone(),
        )
        .await
        {
            let _ = events.send(BackendEvent::Closed {
                tab_id: task_tab,
                reason: format!("{err:#}"),
            });
        }
    });
    BackendTx::Ssh(cmd_tx)
}

pub async fn sample_remote_system(session: Session) -> Result<SystemSnapshot> {
    let (events_tx, _events_rx) = std::sync::mpsc::channel();
    let handle = connect_and_authenticate("remote-metrics", &session, &events_tx).await?;
    let mut channel = handle
        .channel_open_session()
        .await
        .context("open metrics session")?;
    channel
        .exec(true, REMOTE_SYSTEM_PROBE)
        .await
        .context("exec remote metrics probe")?;

    let mut stdout = Vec::new();
    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, ext: _ } => {
                stdout.extend_from_slice(&data);
            }
            ChannelMsg::Close => break,
            _ => {}
        }
    }

    let _ = handle
        .disconnect(Disconnect::ByApplication, "metrics done", "")
        .await;
    let output = String::from_utf8_lossy(&stdout);
    remote_snapshot_from_kv(&output)
}

async fn run_ssh(
    tab_id: String,
    session: Session,
    cols: u16,
    rows: u16,
    mut commands: mpsc::UnboundedReceiver<BackendCommand>,
    events: std::sync::mpsc::Sender<BackendEvent>,
) -> Result<()> {
    let _ = events.send(BackendEvent::Status {
        tab_id: tab_id.clone(),
        text: format!(
            "connecting {}@{}:{}...",
            session.user, session.host, session.port
        ),
    });

    let handle = connect_and_authenticate(&tab_id, &session, &events).await?;

    let mut channel = handle
        .channel_open_session()
        .await
        .context("open session")?;
    channel
        .request_pty(true, "xterm-256color", cols.into(), rows.into(), 0, 0, &[])
        .await
        .context("request pty")?;
    channel.request_shell(true).await.context("request shell")?;

    let _ = events.send(BackendEvent::Status {
        tab_id: tab_id.clone(),
        text: format!("connected {}@{}", session.user, session.host),
    });
    let _ = events.send(BackendEvent::Connected {
        tab_id: tab_id.clone(),
    });

    let exit_reason;
    let mut is_graceful_close = false;

    loop {
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(BackendCommand::Input(bytes)) => {
                        if let Err(err) = channel.data(bytes.as_slice()).await {
                            exit_reason = format!("ssh write error: {err}");
                            break;
                        }
                    }
                    Some(BackendCommand::Resize { cols, rows }) => {
                        let _ = channel.window_change(cols.into(), rows.into(), 0, 0).await;
                    }
                    Some(BackendCommand::Close) | None => {
                        let _ = channel.eof().await;
                        exit_reason = "ssh session closed".to_string();
                        break;
                    }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, ext: _ }) => {
                        let _ = events.send(BackendEvent::Output {
                            tab_id: tab_id.clone(),
                            bytes: data.to_vec(),
                        });
                    }
                    Some(ChannelMsg::ExitStatus { exit_status: _ }) | Some(ChannelMsg::Eof) => {
                        is_graceful_close = true;
                    }
                    Some(ChannelMsg::Close) => {
                        if is_graceful_close {
                            exit_reason = "ssh session closed".to_string();
                        } else {
                            exit_reason = "ssh connection lost (abrupt close)".to_string();
                        }
                        break;
                    }
                    None => {
                        if is_graceful_close {
                            exit_reason = "ssh session closed".to_string();
                        } else {
                            exit_reason = "ssh connection lost (network drop)".to_string();
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = handle
        .disconnect(Disconnect::ByApplication, "bye", "")
        .await;
    let _ = events.send(BackendEvent::Closed {
        tab_id,
        reason: exit_reason,
    });
    Ok(())
}

async fn connect_and_authenticate(
    tab_id: &str,
    session: &Session,
    events: &std::sync::mpsc::Sender<BackendEvent>,
) -> Result<russh::client::Handle<ClientHandler>> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(600)),
        keepalive_interval: Some(std::time::Duration::from_secs(10)),
        keepalive_max: 3,
        ..Default::default()
    });
    let addr = format!("{}:{}", session.host, session.port);
    let _ = events.send(BackendEvent::Status {
        tab_id: tab_id.to_string(),
        text: format!("opening tcp connection to {addr}"),
    });
    let mut handle = client::connect(config, addr.as_str(), ClientHandler)
        .await
        .with_context(|| format!("connect {addr} failed"))?;

    let authed = match session.auth {
        AuthMethod::Password => {
            let _ = events.send(BackendEvent::Status {
                tab_id: tab_id.to_string(),
                text: format!(
                    "connected to {addr}, sending password authentication for {}",
                    session.user
                ),
            });
            handle
                .authenticate_password(&session.user, &session.password)
                .await
                .context("password authentication failed")?
        }
        AuthMethod::Key => {
            let source = key_source_label(session);
            let _ = events.send(BackendEvent::Status {
                tab_id: tab_id.to_string(),
                text: format!("connected to {addr}, loading private key from {source}"),
            });
            let keypair = load_session_private_key(session)?;
            let algorithm = format!("{:?}", keypair.algorithm());
            let _ = events.send(BackendEvent::Status {
                tab_id: tab_id.to_string(),
                text: format!("private key loaded from {source}, algorithm {algorithm}, sending public key authentication for {}", session.user),
            });
            let key = private_key_with_alg(keypair).context("invalid private key")?;
            handle
                .authenticate_publickey(&session.user, key)
                .await
                .with_context(|| {
                    format!(
                        "public key authentication failed for {}@{}:{} using {} ({})",
                        session.user, session.host, session.port, source, algorithm
                    )
                })?
        }
    };

    if !authed {
        let _ = handle
            .disconnect(Disconnect::ByApplication, "auth failed", "")
            .await;
        return Err(anyhow!(
            "{}",
            match session.auth {
                AuthMethod::Password => format!(
                    "authentication failed: server rejected password authentication for {}@{}:{}",
                    session.user, session.host, session.port
                ),
                AuthMethod::Key => format!(
                    "authentication failed: server rejected public key authentication for {}@{}:{} using {}",
                    session.user,
                    session.host,
                    session.port,
                    key_source_label(session)
                ),
            }
        ));
    }

    let _ = events.send(BackendEvent::Status {
        tab_id: tab_id.to_string(),
        text: format!(
            "authentication accepted, opening shell for {}@{}",
            session.user, session.host
        ),
    });

    Ok(handle)
}

fn load_session_private_key(session: &Session) -> Result<PrivateKey> {
    let inline_key = normalize_inline_private_key(&session.private_key_inline);
    let key_path = expand_key_path(session.private_key_path.trim());
    let has_inline = !inline_key.is_empty();
    let has_path = key_path.is_some();

    if !has_inline && !has_path {
        return Err(anyhow!("private key content or path is required"));
    }

    let mut errors = Vec::new();

    if has_inline {
        match decode_secret_key(&inline_key, None) {
            Ok(key) => return Ok(key),
            Err(err) => errors.push(format!("decode private key content: {err}")),
        }
    }

    if let Some(path) = key_path {
        match load_secret_key(path.as_path(), None) {
            Ok(key) => return Ok(key),
            Err(err) => errors.push(format!("load key {}: {err}", path.display())),
        }
    }

    Err(anyhow!(errors.join("; ")))
}

fn private_key_with_alg(keypair: PrivateKey) -> Result<PrivateKeyWithHashAlg> {
    let hash_alg = if keypair.algorithm().is_rsa() {
        Some(HashAlg::Sha512)
    } else {
        None
    };
    Ok(
        PrivateKeyWithHashAlg::new(Arc::new(keypair.clone()), hash_alg)
            .or_else(|_| PrivateKeyWithHashAlg::new(Arc::new(keypair), Some(HashAlg::Sha256)))?,
    )
}

fn normalize_inline_private_key(value: &str) -> String {
    let mut normalized = value
        .trim()
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\r\n", "\n");
    if !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

fn expand_key_path(value: &str) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    if value == "~" {
        return BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return BaseDirs::new().map(|dirs| dirs.home_dir().join(rest));
    }
    Some(Path::new(value).to_path_buf())
}

fn key_source_label(session: &Session) -> String {
    let path = session.private_key_path.trim();
    let has_inline = !session.private_key_inline.trim().is_empty();
    match (!path.is_empty(), has_inline) {
        (true, true) => format!("inline key or {}", path),
        (true, false) => path.to_string(),
        (false, true) => "inline key text".to_string(),
        (false, false) => "unknown key source".to_string(),
    }
}

const REMOTE_SYSTEM_PROBE: &str = r#"sh -lc '
os=$(uname -s 2>/dev/null || echo unknown)

if [ "$os" = "Linux" ] && [ -r /proc/stat ]; then
  cpu_stat() { awk '"'"'/^cpu / { print ($2+$3+$4+$5+$6+$7+$8), $5 }'"'"' /proc/stat 2>/dev/null; }
  net_stat() { awk -F"[: ]+" '"'"'/:/ && $1!="Inter" && $1!="face" { rx += $3; tx += $11 } END { print rx+0, tx+0 }'"'"' /proc/net/dev 2>/dev/null; }

  read cpu_total_1 cpu_idle_1 <<EOF
$(cpu_stat)
EOF
  read net_rx_1 net_tx_1 <<EOF
$(net_stat)
EOF
  sleep 1
  read cpu_total_2 cpu_idle_2 <<EOF
$(cpu_stat)
EOF
  read net_rx_2 net_tx_2 <<EOF
$(net_stat)
EOF

  cpu_delta=$((cpu_total_2 - cpu_total_1))
  idle_delta=$((cpu_idle_2 - cpu_idle_1))
  cpu_percent=$(awk -v total="$cpu_delta" -v idle="$idle_delta" '"'"'BEGIN { if (total <= 0) print "0.00"; else printf "%.2f", ((total-idle)/total)*100 }'"'"')
  mem_total=$(awk '"'"'/^MemTotal:/ {print $2 * 1024}'"'"' /proc/meminfo 2>/dev/null)
  mem_available=$(awk '"'"'/^MemAvailable:/ {print $2 * 1024}'"'"' /proc/meminfo 2>/dev/null)
  swap_total=$(awk '"'"'/^SwapTotal:/ {print $2 * 1024}'"'"' /proc/meminfo 2>/dev/null)
  swap_free=$(awk '"'"'/^SwapFree:/ {print $2 * 1024}'"'"' /proc/meminfo 2>/dev/null)

  echo "CPU_PERCENT=${cpu_percent:-0.00}"
  echo "MEM_TOTAL=${mem_total:-0}"
  echo "MEM_USED=$(( ${mem_total:-0} - ${mem_available:-0} ))"
  echo "SWAP_TOTAL=${swap_total:-0}"
  echo "SWAP_USED=$(( ${swap_total:-0} - ${swap_free:-0} ))"
  echo "NET_RX=$(( ${net_rx_2:-0} - ${net_rx_1:-0} ))"
  echo "NET_TX=$(( ${net_tx_2:-0} - ${net_tx_1:-0} ))"
  df -kP 2>/dev/null | awk "NR > 1 && \$1 !~ /^(tmpfs|devtmpfs|ramfs|overlay|aufs)\$/ { printf \"DISK=%s\t%s\t%s\n\", \$6, \$4 * 1024, \$2 * 1024 }" | head -n 6
  exit 0
fi

if [ "$os" = "Darwin" ]; then
  net_stat() { netstat -ibn 2>/dev/null | awk '"'"'NR > 1 && $7 ~ /^[0-9]+$/ && $10 ~ /^[0-9]+$/ { rx += $7; tx += $10 } END { print rx+0, tx+0 }'"'"'; }

  read net_rx_1 net_tx_1 <<EOF
$(net_stat)
EOF
  sleep 1
  read net_rx_2 net_tx_2 <<EOF
$(net_stat)
EOF

  cpu_percent=$(top -l 2 -n 0 -s 1 2>/dev/null | awk -F"[:,% ]+" '"'"'/CPU usage:/ { user=$3; sys=$5 } END { if (user == "" && sys == "") print "0.00"; else printf "%.2f", user + sys }'"'"')
  mem_total=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
  pagesize=$(sysctl -n hw.pagesize 2>/dev/null || echo 4096)
  vm_output=$(vm_stat 2>/dev/null)
  pages_active=$(printf "%s\n" "$vm_output" | awk '"'"'/Pages active/ { gsub("\\.","",$3); print $3+0 }'"'"')
  pages_wired=$(printf "%s\n" "$vm_output" | awk '"'"'/Pages wired down/ { gsub("\\.","",$4); print $4+0 }'"'"')
  pages_compressed=$(printf "%s\n" "$vm_output" | awk '"'"'/Pages occupied by compressor/ { gsub("\\.","",$5); print $5+0 }'"'"')
  pages_speculative=$(printf "%s\n" "$vm_output" | awk '"'"'/Pages speculative/ { gsub("\\.","",$3); print $3+0 }'"'"')
  mem_used=$(( (${pages_active:-0} + ${pages_wired:-0} + ${pages_compressed:-0} + ${pages_speculative:-0}) * ${pagesize:-4096} ))
  swap_line=$(sysctl vm.swapusage 2>/dev/null || true)
  swap_used=$(printf "%s\n" "$swap_line" | awk -F"[= ,]+" '"'"'
    function mult(unit) { return unit=="K"?1024:(unit=="M"?1048576:(unit=="G"?1073741824:(unit=="T"?1099511627776:1))) }
    /used/ { value=$4; unit=substr(value, length(value), 1); sub(/[A-Za-z]+$/, "", value); printf "%.0f", value * mult(unit) }'"'"')
  swap_total=$(printf "%s\n" "$swap_line" | awk -F"[= ,]+" '"'"'
    function mult(unit) { return unit=="K"?1024:(unit=="M"?1048576:(unit=="G"?1073741824:(unit=="T"?1099511627776:1))) }
    /used/ && /free/ { used=$4; free=$8; unit1=substr(used, length(used), 1); unit2=substr(free, length(free), 1); sub(/[A-Za-z]+$/, "", used); sub(/[A-Za-z]+$/, "", free); printf "%.0f", (used * mult(unit1)) + (free * mult(unit2)) }'"'"')

  echo "CPU_PERCENT=${cpu_percent:-0.00}"
  echo "MEM_TOTAL=${mem_total:-0}"
  echo "MEM_USED=${mem_used:-0}"
  echo "SWAP_TOTAL=${swap_total:-0}"
  echo "SWAP_USED=${swap_used:-0}"
  echo "NET_RX=$(( ${net_rx_2:-0} - ${net_rx_1:-0} ))"
  echo "NET_TX=$(( ${net_tx_2:-0} - ${net_tx_1:-0} ))"
  df -kP 2>/dev/null | awk "NR > 1 && \$1 !~ /^(devfs|tmpfs|devtmpfs|ramfs|overlay|aufs)\$/ { printf \"DISK=%s\t%s\t%s\n\", \$6, \$4 * 1024, \$2 * 1024 }" | head -n 6
  exit 0
fi

echo "CPU_PERCENT=0.00"
echo "MEM_TOTAL=0"
echo "MEM_USED=0"
echo "SWAP_TOTAL=0"
echo "SWAP_USED=0"
echo "NET_RX=0"
echo "NET_TX=0"
'"#;

struct ClientHandler;

#[async_trait]
impl Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
