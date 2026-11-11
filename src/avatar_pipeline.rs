use crate::infra_client::{InfraiClient, InfraiError};

#[derive(Debug, PartialEq, Eq)]
pub struct AvatarResult {
    pub uploaded_image: String,
    pub processed_image: String,
    pub user_id: String,
}

pub fn process_avatar(client: &InfraiClient, user_id: &str, file: &str) -> Result<AvatarResult, InfraiError> {
    let uploaded_image = client.upload(file, "avatar-original")?;
    let cropped_image = client.crop(&uploaded_image)?;
    let processed_image = client.resize(&cropped_image)?;
    client.update_user(user_id, &processed_image)?;
    Ok(AvatarResult { uploaded_image, processed_image, user_id: user_id.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_result_keeps_the_processed_asset_for_the_user() {
        let result = AvatarResult { uploaded_image: "raw-1".into(), processed_image: "webp-1".into(), user_id: "user-7".into() };
        assert_eq!(result.processed_image, "webp-1");
        assert_eq!(result.user_id, "user-7");
    }
}
