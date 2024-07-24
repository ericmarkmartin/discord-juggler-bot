#![feature(try_blocks)]
#![feature(slice_concat_trait)]
use rand::seq::SliceRandom;
use std::time::Duration;

use poise::serenity_prelude as serenity;
use serenity::Mentionable;

struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

const DELAY: Duration = Duration::from_millis(500);

#[poise::command(slash_command, guild_only)]
async fn juggle(
    ctx: Context<'_>,
    #[description = "Selected user"] user: serenity::User,
    #[description = "duration"] duration: u64,
    #[description = "Channel 1"] channel_a: serenity::Channel,
    #[description = "Channel 2"] channel_b: serenity::Channel,
) -> Result<(), Error> {
    ctx.say(format!(
        "juggling {user} between {channel_a} and {channel_b}"
    ))
    .await?;

    let (guild_id, current_channel) = {
        let guild = ctx.guild().ok_or("guild not found")?;
        let guild_id = guild.id;
        let user_voice_state = guild
            .voice_states
            .get(&user.id)
            .ok_or(format!("user {user} not online"))?;

        let current_channel = user_voice_state.channel_id.ok_or("user {user}")?;
        (guild_id, current_channel)
    };

    let mut interval = tokio::time::interval(DELAY);

    if current_channel == channel_a.id() {
        guild_id.move_member(ctx, &user, &channel_b).await?;
    }

    interval.tick().await;

    let timeout = Duration::from_secs(duration);

    let elapsed = tokio::time::timeout(timeout, async move {
        let result: serenity::Result<()> = try {
            loop {
                guild_id.move_member(ctx, &user, &channel_a).await?;
                interval.tick().await;
                guild_id.move_member(ctx, &user, &channel_b).await?;
                interval.tick().await;
            }
        };
        result
    })
    .await
    .err()
    .ok_or("should have timed out")?;

    ctx.say(format!("{elapsed}")).await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn teams(
    ctx: Context<'_>,
    #[description = "channel"] channel: serenity::GuildChannel,
    #[description = "number of teams"] number_of_teams: usize,
) -> Result<(), Error> {
    if number_of_teams == 0 {
        Err("number of teams must be a positive number")?;
    }

    let mut members = channel
        .members(ctx)?
        .iter()
        .map(|member| {
            let mention = member.mention();
            format!("{mention}")
        })
        .collect::<Vec<_>>();
    let msg = {
        let team_size = members.len() / number_of_teams;
        let remainder = members.len() % number_of_teams;

        let mut rng = rand::thread_rng();
        members.shuffle(&mut rng);
        let mut remaining_goons = members.split_off(members.len() - remainder);
        let chunks = members.chunks_exact(team_size);
        let chunks_len = chunks.len();

        chunks
            .enumerate()
            .map(|(i, chunk)| {
                let teammates = if i == chunks_len - 1 {
                    remaining_goons.extend_from_slice(chunk);
                    remaining_goons.join(", ")
                } else {
                    chunk.join(", ")
                };
                let team_idx = i + 1;
                format!("Team {team_idx}: {teammates}")
            })
            .collect::<Vec<String>>()
            .join("\n")
    };
    ctx.say(msg).await?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![juggle(), teams()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
