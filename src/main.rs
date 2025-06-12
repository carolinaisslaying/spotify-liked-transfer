use std::env::var;

use dotenv::dotenv;
use futures::stream::TryStreamExt;

use rspotify::clients::OAuthClient;
use rspotify::model::{TrackId, UserId};
use rspotify::{prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let client_id = var("CLIENT_ID").unwrap();
    let client_secret = var("CLIENT_SECRET").unwrap();
    let creds = Credentials::new(&client_id, &client_secret);

    let oauth = OAuth {
        redirect_uri: "http://127.0.0.1:80".to_string(),
        scopes: scopes!("user-library-read playlist-modify-public"),
        ..Default::default()
    };

    let spotify = AuthCodeSpotify::new(creds, oauth);

    // Obtaining the access token
    let url = spotify.get_authorize_url(false).unwrap();
    // This function requires the `cli` feature enabled.
    spotify.prompt_for_token(&url).await.unwrap();

    let user_id = &var("USER_ID")?;

    let playlist = spotify
        .user_playlist_create(
            UserId::from_id(user_id).unwrap(),
            "Liked",
            Some(true),
            None,
            Some("<3"),
        )
        .await?;

    let mut saved_tracks = spotify.current_user_saved_tracks(None);

    let mut done = false;
    // Batch the tracks to add them in bulk in batches of 50
    let mut tracks_batch: Vec<PlayableId> = Vec::new();

    println!("Fetching tracks");
    while !done {
        let track = saved_tracks.try_next().await;
        // If that fails, we're done
        if let Err(_) = track {
            println!("Something went wrong, assuming done");
            done = true;
            continue;
        }

        let track = track.unwrap();
        if let None = track {
            println!("Finished fetching tracks");
            done = true;
            continue;
        }

        // Add the track to the playlist
        let track = track.unwrap();
        let track_id: Option<TrackId<'_>> = track.track.id;
        if let None = track_id {
            println!("Got track, but it had no ID, local?");
            continue;
        }

        let playable_id = PlayableId::Track(track_id.unwrap());

        tracks_batch.push(playable_id);
        if tracks_batch.len() % 100 == 0 {
            println!("Fetched {} tracks", tracks_batch.len());
        }
    }

    // Print amount of tracks
    println!("Amount of tracks to add: {}", tracks_batch.len());

    let playlist_id = playlist.id;
    let total = tracks_batch.len();
    let mut current = 0;

    while !tracks_batch.is_empty() {
        let tracks_left = tracks_batch.len();
        let small_tracks_batch: Vec<PlayableId> = tracks_batch
            // Drain the first 100 tracks, unless there are less than 100 left
            .drain(0..std::cmp::min(tracks_left, 100))
            .collect();
        let length = small_tracks_batch.len();
        spotify
            .playlist_add_items(playlist_id.clone(), small_tracks_batch, None)
            .await?;
        current += length;
        println!("Added {}/{} tracks", current, total);
    }

    Ok(())
}
