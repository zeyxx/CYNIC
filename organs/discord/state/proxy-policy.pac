function FindProxyForURL(url, host) {
  host = host.toLowerCase();

  if (host == "localhost" || host == "127.0.0.1") {
    return "DIRECT";
  }

  if (host == "mitm.it" || shExpMatch(host, "*.mitm.it")) {
    return "PROXY 127.0.0.1:8890; DIRECT";
  }

  if (host == "discord.com") {
    if (shExpMatch(url, "https://discord.com/api/*/channels/*/messages*")) {
      return "PROXY 127.0.0.1:8890; DIRECT";
    }
    if (shExpMatch(url, "https://discord.com/api/*/guilds/*/channels*")) {
      return "PROXY 127.0.0.1:8890; DIRECT";
    }
    if (shExpMatch(url, "https://discord.com/api/*/guilds/*/threads*")) {
      return "PROXY 127.0.0.1:8890; DIRECT";
    }
    return "DIRECT";
  }

  if (shExpMatch(host, "*.discord.com")) {
    return "DIRECT";
  }

  if (host == "discordapp.com") {
    if (shExpMatch(url, "https://discordapp.com/api/*/channels/*/messages*")) {
      return "PROXY 127.0.0.1:8890; DIRECT";
    }
    return "DIRECT";
  }

  return "DIRECT";
}
