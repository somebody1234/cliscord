use clap::Clap;
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
struct Guild {
  id: String,
  name: String,
}

#[derive(Deserialize)]
struct Channel {
  id: String,
  name: Option<String>,
}

#[derive(Clap)]
struct Opts {
   #[clap(short, long)]
  server: Option<String>,
   #[clap(short, long)]
  channel: Option<String>,
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
  let token_string = std::env::var("DISCORD_TOKEN").expect("token");
  let token = token_string.as_str();
  let opts = Opts::parse();
  let mut server_name: Option<String> = opts.server.map(|s| s.to_lowercase());
  let mut channel_name: Option<String> = opts.channel.map(|s| s.to_lowercase());
  if let Ok(contents) = std::fs::read_to_string("config") {
    let parts: Vec<&str> = contents.split('\n').collect();
    if parts[0].len() > 0 {
      server_name = Some(String::from(parts[0]).to_lowercase());
    }
    if parts.len() > 0 && parts[1].len() > 0 {
      channel_name = Some(String::from(parts[1]).to_lowercase());
    }
  }
  let client = reqwest::Client::new();
  let servers = client.get("https://discord.com/api/v6/users/@me/guilds")
    .header("authorization", token)
    .send()
    .await?
    .json::<std::vec::Vec<Guild>>()
    .await?;
  let server_name = server_name.expect("missing server name");
  let channel_name = channel_name.expect("missing channel name");
  let mut server_id: Option<String> = None;
  let mut channel_id: Option<String> = None;
  for server in servers {
    if server.name.to_lowercase() == server_name {
      server_id = Some(server.id);
    }
  }
  if server_id == None {
    panic!("invalid server");
  } else {
    let channels = client.get(format!("https://discord.com/api/v6/guilds/{}/channels", server_id.unwrap()).as_str())
      .header("authorization", token)
      .send()
      .await?
      .json::<std::vec::Vec<Channel>>()
      .await?;
    for channel in channels {
      if let Some(name) = channel.name {
        if name.to_lowercase() == channel_name {
          channel_id = Some(channel.id);
        }
      }
    }
  }
  if channel_id == None {
    panic!("invalid channel");
  } else {
    let mut buffer = String::new();
    if let Ok(_) = std::io::stdin().read_to_string(&mut buffer) {}
    client.post(format!("https://discord.com/api/v6/channels/{}/messages", channel_id.unwrap()).as_str())
      .header("authorization", token)
      .form(&[("content", buffer.as_str())])
      .send()
      .await?;
  }
  if let Ok(_) = std::fs::write("config", format!("{}\n{}", server_name, channel_name)) {}
  Ok(())
}

