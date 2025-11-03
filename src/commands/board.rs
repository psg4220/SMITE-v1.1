use serenity::model::channel::Message;
use serenity::prelude::Context;

use crate::services::board_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    board_service::list_currencies(ctx, msg, args).await
}
