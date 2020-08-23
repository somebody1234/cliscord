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
  #[clap(short, long)]
  message: Option<String>,
  #[clap(short, long)]
  token: Option<String>,
  #[clap(short, long)]
  block: bool,
  path: Option<String>,
}

#[cfg(feature = "filetype")]
fn false_matcher(_buf: &[u8]) -> bool { false }

#[cfg(feature = "filetype")]
fn infer(buf: &[u8]) -> infer::Type {
  if let Some(type_) = infer::get(&buf) {
    type_
  } else if buf.starts_with(b"#!/usr/bin/") || buf.starts_with(b"#!/bin/") {
    let lang = if buf.starts_with(b"#!/usr/bin/env ") {
      &buf[b"#!/usr/bin/env ".len()..]
    } else if buf.starts_with(b"#!/bin/env ") {
      &buf[b"#!/bin/env ".len()..]
    } else if buf.starts_with(b"#!/usr/bin") {
      &buf[b"#!/usr/bin/".len()..]
    } else {
      &buf[b"#!/bin/".len()..]
    };
    match std::str::from_utf8(&lang).expect("shebang not in utf-8").to_lowercase().as_str() {
      "node" | "deno" | "spidermonkey" | "v8" => infer::Type::new(infer::MatcherType::APP, "application/javascript", "js", false_matcher),
      "ts-node" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "ts", false_matcher),
      // NOTE: i could include some minor versions; however i doubt they're used often enough, esp. in shebang lines.
      "python" | "python2" | "python3" | "pypy" | "pypy2" | "pypy3" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "py", false_matcher),
      "ruby" | "irb" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "rb", false_matcher),
      "kotlinc" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "kt", false_matcher),
      "php" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "php", false_matcher),
      "perl" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "perl", false_matcher),
      "raku" | "perl6" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "perl", false_matcher),
      "lua" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "lua", false_matcher),

      "sh" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "sh", false_matcher),
      "bash" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "bash", false_matcher),
      "zsh" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "zsh", false_matcher),
      "fish" => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "fish", false_matcher),
      _ => infer::Type::new(infer::MatcherType::TEXT, "text/plain", "txt", false_matcher)
    }
  } else {
    infer::Type::new(infer::MatcherType::TEXT, "text/plain", "txt", false_matcher)
  }
}

#[cfg(feature = "token")]
fn u8_alnum(c: u8) -> bool {
  (c >= b'0' && c <= b'9') || (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') || c == b'_'
}

#[cfg(feature = "token")]
fn tok(path: String) -> std::io::Result<String> {
  for entry in std::fs::read_dir(path)? {
    let entry = entry?;
    let path = entry.path();
    if path.file_name().map(|v| v.to_str()).flatten().map_or(false, |v| v.ends_with(".ldb")) {
      let contents = std::fs::read(path)?;
      let mut i = 0u8;
      // poor man's state machine
      for (j, c) in contents.iter().enumerate() {
        let c = *c;
        if i < 24 {
          if u8_alnum(c) { i += 1; } else { i = 0; }
        } else if i == 24 {
          if c == b'.' { i += 1; } else if !u8_alnum(c) { i = 0; }
        } else if i < 31 {
          if u8_alnum(c) { i += 1; } else { i = 0; }
        } else if i == 31 {
          if c == b'.' { i += 1; } else { i = 0; }
        } else {
          if u8_alnum(c) { i += 1; } else { i = 0; }
        }
        if i == 59 {
          return Ok(std::str::from_utf8(&contents[j-58..j+1]).unwrap().to_string());
        }
      }
    }
  }
  Err(std::io::Error::new(std::io::ErrorKind::Other, "token not found"))
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
  let opts = Opts::parse();
  let token_string = if cfg!(feature = "token") {
    opts.token.ok_or_else(|| std::env::var("DISCORD_TOKEN"))
      .or_else(|_| tok(dirs::config_dir().expect("config dir unknown").join("discord/Local Storage/leveldb").to_string_lossy().to_string()))
      .or_else(|_| tok(dirs::config_dir().expect("config dir unknown").join("google-chrome/Default/Local Storage/leveldb/").to_string_lossy().to_string()))
      .expect("discord token not found, set environment variable DISCORD_TOKEN, pass in as argument, or login to discord on desktop app or on chrome")
  } else {
    opts.token.ok_or_else(|| std::env::var("DISCORD_TOKEN"))
      .expect("discord token not found, set environment variable DISCORD_TOKEN or pass in as argument")
  };
  let token = token_string.as_str();
  let mut server_name: Option<String> = opts.server.map(|s| s.to_lowercase());
  let mut channel_name: Option<String> = opts.channel.map(|s| s.to_lowercase());
  if let Ok(contents) = std::fs::read_to_string("config") {
    let parts: Vec<&str> = contents.split('\n').collect();
    if parts[0].len() > 0 {
      server_name = server_name.or(Some(parts[0].to_string().to_lowercase()));
    }
    if parts.len() > 0 && parts[1].len() > 0 {
      channel_name = channel_name.or(Some(parts[1].to_string().to_lowercase()));
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
    let mut buffer = Vec::new();
    if let Some(path) = &opts.path {
      if let Ok(vec) = std::fs::read(path) {
        buffer = vec;
      }
    } else if let Some(message) = opts.message {
      buffer = message.as_str().as_bytes().into();
    } else if let Ok(_) = std::io::stdin().read_to_end(&mut buffer) {
    }
    if buffer.len() > 0 {
      if cfg!(feature = "filetype") {
        let type_ = infer(&buffer);
        let is_text = type_.matcher_type() == infer::MatcherType::TEXT || (type_.matcher_type() == infer::MatcherType::APP && type_.mime_type() == "application/javascript");
        let mut send_file = false;
        if opts.block {
          if buffer.len() + 8 + type_.extension().len() > 2000 {
            panic!("content too long; cannot put in block");
          } else if !is_text {
            panic!("non-text files cannot be inserted into code block");
          }
          let mut extension = type_.extension().to_string();
          if let Some(path) = &opts.path {
            extension = std::path::Path::new(path)
              .extension().expect("filename not valid")
              .to_str().expect("filename is not valid utf-8")
              .to_string();
          }
          buffer = format!("```{}\n{}\n```", extension, std::str::from_utf8(&buffer).expect("message not utf-8")).as_bytes().into();
        } else if is_text && buffer.len() < 2000 {
          // text message
        } else {
          // file attachment; with appropriate embed if there is one
          send_file = true;
        }
        if send_file {
          let mut file_part = reqwest::multipart::Part::bytes(buffer)
            .mime_str(type_.mime_type())
            .expect("could not set embed file mimetype");
          let file_name = match &opts.path {
            Some(path) => std::path::Path::new(path)
              .file_name().expect("filename not valid")
              .to_str().expect("filename is not valid utf-8")
              .to_string(),
            None => format!("message.{}", type_.extension()),
          };
          file_part = file_part.file_name(file_name.clone());
          let form = reqwest::multipart::Form::new()
            .part("file", file_part);
          client.post(format!("https://discord.com/api/v6/channels/{}/messages", channel_id.unwrap()).as_str())
            .header("authorization", token)
            .multipart(form)
            .send()
            .await?;
        } else {
          client.post(format!("https://discord.com/api/v6/channels/{}/messages", channel_id.unwrap()).as_str())
            .header("authorization", token)
            .form(&[("content", std::str::from_utf8(&buffer).expect("message not utf-8"))])
            .send()
            .await?;
        }
      } else {
        client.post(format!("https://discord.com/api/v6/channels/{}/messages", channel_id.unwrap()).as_str())
          .header("authorization", token)
          .form(&[("content", std::str::from_utf8(&buffer).expect("message not utf-8"))])
          .send()
          .await?;
      }
    }
  }
  if let Ok(_) = std::fs::write("config", format!("{}\n{}", server_name, channel_name)) {
  }
  Ok(())
}

