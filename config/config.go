package config

import (
    "fmt"
    "github.com/BurntSushi/toml"
)

type Config struct {
    Token string `toml:"token"`
    Messages []Message `toml:"messages"`
    Roles []Role `toml:"roles"`
}

type Message struct {
    ChannelID string `toml:"channel"`
    ID string `toml:"id"`
}

type Role struct {
    EmojiName string `toml:"name"`
    EmojiID string `toml:"id"`
    ID string `toml:"role"`
}

func (r Role) Format() string {
    return fmt.Sprintf("%s:%s", r.EmojiName, r.EmojiID)
}

func Load(data []byte) (*Config, error) {
    var conf Config
    _, err := toml.Decode(string(data), &conf)
    return &conf, err
}
