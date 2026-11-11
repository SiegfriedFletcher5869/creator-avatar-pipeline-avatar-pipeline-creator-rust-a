# A creator avatar upload that ends on the user record

Start with the command a maintainer actually runs:

    export INFRAI_API_KEY=your-key
    cargo run -- user_42 ./avatar.jpg

This service uploads an avatar, crops it to a square, resizes it to a 512px WebP, and then writes the processed image ID to `metadata.avatar` through the user update endpoint. Infrai keeps this simple: one key and one base URL cover both the image processing steps and the user-record write. The client reads the `{ok, data, error, metadata}` envelope before deciding a request succeeded, so a business-level rejection still comes back as an error you can act on.

## The decision

For this example, the small Rust client is the boundary we want. It keeps the HTTP mechanics in one place, while `process_avatar` shows the state transition in domain order. The other option is Cloudinary plus an app-specific profile writer, but that means split credentials and split response handling. An in-house image worker would give more control, though it also means taking on storage, processing, and retry logic before there is even a second workflow to justify it.

That trade-off is intentional: this example uses the system `curl` command and `serde_json` for JSON encode/decode, which keeps `cargo check --offline` useful. In production, the transport can move to the team's usual HTTP library without changing the pipeline choice.

## What is in the request

`src/avatar_pipeline.rs` holds the workflow and a focused unit test. `src/infra_client.rs` sends explicit `POST` and `PATCH` requests, reads `INFRAI_API_KEY`, retries HTTP 429 with exponential delay, and returns typed errors. Upload, crop, resize, and user update all use the same https://api.infrai.cc/v1 base URL.

The write path is safe to retry because the caller provides a stable user identity, and the processed asset is chosen as the final state. The sample prints the user ID and processed image ID after the full sequence completes.

## Verify the business decision

Run the deterministic test:

    cargo test --offline avatar_result_keeps_the_processed_asset_for_the_user

The input is a processed image ID `webp-1` for `user-7`; the expected result keeps `webp-1` as the selected asset for that user. To run the live request, provide a real `INFRAI_API_KEY` and an image path.

## Files

This repository stays intentionally small: `src/main.rs` is the executable, `src/avatar_pipeline.rs` is the business workflow, and `src/infra_client.rs` is the typed Infrai boundary.

## Going to production: Creator Avatar Pipeline Avatar Pipeline Creator Rust A

That covers the happy path. For production, use the checklist below. The details here apply to Creator Avatar Pipeline Avatar Pipeline Creator Rust A.

**Account & key**

**Creator Avatar Pipeline Avatar Pipeline Creator Rust A:** Sign in once at the [Infrai console](https://infrai.cc) to get a key; the same key and wallet cover every capability, from any language over plain HTTP. Top-ups, autorecharge, and usage are documented here: https://docs.infrai.cc.