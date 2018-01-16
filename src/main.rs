extern crate discord;
extern crate config;

use discord::model::{ChannelId, MessageId, ReactionEmoji, EmojiId, RoleId, UserId, ServerId, Channel};
use discord::model::Event;

const REACTION_CAP: usize = 20;
const X: &str = "❌";

struct Role {
	role_id: RoleId,
	emoji: ReactionEmoji,
}

fn main() {
	let mut cfg = config::Config::default();
	cfg.merge(config::File::with_name("config.toml")).unwrap();

	let token = cfg.get_str("token").unwrap();
	let dc = discord::Discord::from_bot_token(&token).unwrap();
	
	let roles: Vec<Role> = cfg.get_array("roles").unwrap().into_iter()
		.map(|r| r.into_table().unwrap())
		.map(|m| Role{
			role_id: RoleId(m.get("role").unwrap().clone().try_into().unwrap()),
			emoji: ReactionEmoji::Custom {
				name: m.get("name").unwrap().clone().into_str().unwrap(),
				id: EmojiId(m.get("id").unwrap().clone().try_into().unwrap()),
			}
		}).collect();
	let mut i: i32 = -1;

	println!("Setting up selectors...");
	let channels: Vec<(ChannelId, MessageId)> = cfg.get_array("messages").unwrap().into_iter()
			.map(|r| r.into_table().unwrap())
			.map(|m| (ChannelId(m.get("channel").unwrap().clone().try_into().unwrap()),
					  MessageId(m.get("id").unwrap().clone().try_into().unwrap()))).collect();
	for &(channel_id, message_id) in &channels {
		let message = dc.get_message(channel_id, message_id).unwrap();
	
		// clears the message of reactions
		for reaction in message.reactions {
			let mut count = reaction.count;
			loop {
				let reactions = dc.get_reactions(channel_id, message_id, reaction.emoji.clone(), None, None).unwrap();
				for user in reactions {
					dc.delete_reaction(channel_id, message_id, Some(user.id), reaction.emoji.clone()).unwrap();
					count -= 1;
				}
				
				if count == 0 {
					break;
				}
			}
		}
		
		for _ in 0 .. REACTION_CAP {
			if i >= roles.len() as i32 {
				break;
			} else if i == -1 {
				// clear role
				dc.add_reaction(channel_id, message_id, ReactionEmoji::Unicode(X.to_string())).unwrap();
			} else {
				dc.add_reaction(channel_id, message_id, roles[i as usize].emoji.clone()).unwrap();
			}
			
			i += 1;
		}
	}

	// main loop
	println!("Starting event loop.");
	let (mut conn, _) = dc.connect().unwrap();
	loop {
		let event = match conn.recv_event() {
			Ok(event) => event,
			Err(err) => {
				println!("[Warning] Receive error: {:?}", err);
				continue;
			},
		};
		//println!("{:?}", event);
		
		if let Event::ReactionAdd(reaction) = event {
			for &(channel_id, message_id) in &channels {
				if channel_id == reaction.channel_id && message_id == reaction.message_id {
					for role in &roles {
						if same_emoji(&reaction.emoji, &role.emoji) {
							let channel = match dc.get_channel(reaction.channel_id) {
								Ok(channel) => match channel {
									Channel::Public(channel) => channel,
									_ => break,
								},
								Err(err) => {
									println!("[Warning] Receive error: {:?}", err);
									break;
								},
							};
						
							match give_role(&dc, &roles, channel.server_id, reaction.user_id, Some(role.role_id)) {
								Ok(()) => (),
								Err(err) => {
									println!("[Warning] Receive error: {:?}", err);
									break;
								},
							};
							
							match dc.delete_reaction(channel_id, message_id, Some(reaction.user_id), reaction.emoji) {
								Ok(()) => (),
								Err(err) => {
									println!("[Warning] Receive error: {:?}", err);
									break;
								},
							};
							
							break;
						} else if same_emoji(&reaction.emoji, &ReactionEmoji::Unicode(X.to_string())) {
							let channel = match dc.get_channel(reaction.channel_id) {
								Ok(channel) => match channel {
									Channel::Public(channel) => channel,
									_ => break,
								},
								Err(err) => {
									println!("[Warning] Receive error: {:?}", err);
									break;
								},
							};
							
							match give_role(&dc, &roles, channel.server_id, reaction.user_id, None) {
								Ok(()) => (),
								Err(err) => {
									println!("[Warning] Receive error: {:?}", err);
									break;
								},
							};
							
							match dc.delete_reaction(channel_id, message_id, Some(reaction.user_id), reaction.emoji) {
								Ok(()) => (),
								Err(err) => {
									println!("[Warning] Receive error: {:?}", err);
									break;
								},
							};
							
							break;
						}
					}
				
					break;
				}
			}
		}
	}
}

fn same_emoji(a: &ReactionEmoji, b: &ReactionEmoji) -> bool {
	use ReactionEmoji::*;

	match *a {
		Unicode(ref u1) => match *b {
			Unicode(ref u2) => u1 == u2,
			Custom{name: _, id: _} => false,
		},
		Custom{name: ref n1, id: ref i1} => match *b {
			Unicode(_) => false,
			Custom{name: ref n2, id: ref i2} => n1==n2 && i1==i2,
		},
	}
}

fn give_role(dc: &discord::Discord, roles: &Vec<Role>, server: ServerId, user: UserId, role: Option<RoleId>) -> discord::Result<()> {
	let member = dc.get_member(server, user)?;
	let mut user_roles = Vec::<RoleId>::new();
	
	for user_role in member.roles {
		let mut a = true;
		for role in roles {
			if user_role == role.role_id {
				a = false;
				break;
			}
		}
		
		if a {
			user_roles.push(user_role);
		}
	}
	match role {
		Some(role) => {
			user_roles.push(role);
			println!("Granting role {} to user {}", role.0, user.0);
		},
		None => {
			println!("Clearing role from user {}", user.0);
		},
	};
	
	dc.edit_member_roles(server, user, &user_roles)
}
