package main

import (
    "io/ioutil"
    "os"
    "os/signal"
    "syscall"
    "fmt"

    "github.com/difarem/discord-emote-giveme-bot/config"

    "github.com/bwmarrin/discordgo"
)

func check(err error) {
    if err != nil {
        panic(err)
    }
}

var conf *config.Config
var messages map[config.Message]bool
var roles map[string]string
var givemes map[string]bool
var me *discordgo.User

func main() {
    conf_data, err := ioutil.ReadFile("config.toml")
    check(err)

    conf, err = config.Load(conf_data)
    check(err)

    roles = make(map[string]string)
    givemes = make(map[string]bool)
    for _, role := range conf.Roles {
        roles[role.Format()] = role.ID
        givemes[role.ID] = true
    }
    messages = make(map[config.Message]bool)
    for _, msg := range conf.Messages {
        messages[msg] = true
    }

    dg, err := discordgo.New(conf.Token)
    check(err)

    me, err = dg.User("@me")
    check(err)

    dg.AddHandler(ready)
    dg.AddHandler(reactionAdd)

    check(dg.Open())
    sc := make(chan os.Signal, 1)
    signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
    <-sc

    dg.Close()
}

func ready(s *discordgo.Session, ev *discordgo.Ready) {
    fmt.Printf("connected as %s\n", ev.User.Username)
    check(setup(s))
}

func setup(s *discordgo.Session) error {
    const REACTION_CAP = 20
    println("setting up reactions...")

    for _, msg := range conf.Messages {
        err := s.MessageReactionsRemoveAll(
            msg.ChannelID,
            msg.ID,
        )
        if err != nil {
            return err
        }
    }

    err := s.MessageReactionAdd(
        conf.Messages[0].ChannelID,
        conf.Messages[0].ID,
        "❌",
    )
    if err != nil {
        return err
    }

    i := 0
    j := 1
    k := 0
    for k < len(conf.Roles) {
        err = s.MessageReactionAdd(
            conf.Messages[i].ChannelID,
            conf.Messages[i].ID,
            conf.Roles[k].Format(),
        )
        if err != nil {
            return err
        }

        j += 1
        k += 1

        if j >= REACTION_CAP {
            i += 1
            j = 0
        }
    }

    return nil
}

func reactionAdd(s *discordgo.Session, ev *discordgo.MessageReactionAdd) {
    if ev.UserID == me.ID {
        return
    }

    msg := config.Message {
        ChannelID: ev.ChannelID,
        ID: ev.MessageID,
    }
    if _, ok := messages[msg]; !ok {
        return
    }

    err := s.MessageReactionRemove(ev.ChannelID, ev.MessageID, ev.Emoji.APIName(), ev.UserID)
    if err != nil {
        fmt.Printf("could not remove reaction: %s\n", err)
        return
    }

    channel, err := s.Channel(ev.ChannelID)
    if err != nil {
        fmt.Printf("could not query channel: %s\n", err)
        return
    }

    member, err := s.GuildMember(channel.GuildID, ev.UserID)
    if err != nil {
        fmt.Printf("could not query guild member: %s\n", err)
        return
    }

    var newRoles []string
    for _, role := range member.Roles {
        if _, ok := givemes[role]; !ok {
            newRoles = append(newRoles, role)
        }
    }

    if role, ok := roles[ev.Emoji.APIName()]; ok {
        fmt.Printf("granting role %s to user %s [%s]\n", role, member.User.Username, member.User.ID)
        newRoles = append(newRoles, role)
    } else {
        fmt.Printf("clearing role from user %s [%s]\n", member.User.Username, member.User.ID)
    }

    err = s.GuildMemberEdit(channel.GuildID, ev.UserID, newRoles)
    if err != nil {
        fmt.Printf("could not edit member roles: %s\n", err)
        return
    }

}
