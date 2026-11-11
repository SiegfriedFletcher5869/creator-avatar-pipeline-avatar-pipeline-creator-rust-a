mod avatar_pipeline;
mod infra_client;

use avatar_pipeline::process_avatar;
use infra_client::InfraiClient;

fn main() {
    let mut args = std::env::args().skip(1);
    let user_id = args.next().unwrap_or_else(|| "user-demo".to_string());
    let file = args.next().unwrap_or_else(|| "avatar.jpg".to_string());
    let client = match InfraiClient::from_env() {
        Ok(client) => client,
        Err(error) => { eprintln!("{error}"); std::process::exit(2); }
    };
    match process_avatar(&client, &user_id, &file) {
        Ok(result) => println!("avatar updated: user={} image={}", result.user_id, result.processed_image),
        Err(error) => { eprintln!("avatar update rejected: {error}"); std::process::exit(1); }
    }
}
