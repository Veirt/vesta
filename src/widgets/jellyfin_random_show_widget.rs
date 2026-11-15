use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{VestaError, VestaResult};
use crate::widget_system::{DynamicOption, WidgetContext, WidgetField, WidgetHandler, WidgetQuery};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Jellyfin base URL, e.g. `https://jellyfin.example.com`.
    pub url: String,
    /// Jellyfin API key stored in config.toml; protect that file.
    pub key: String,
    /// Media library to draw the random series from; empty selects all.
    #[serde(default)]
    pub library: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Series {
    #[serde(alias = "Id")]
    id: String,
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "Overview", default)]
    overview: Option<String>,
    #[serde(alias = "CommunityRating", default)]
    community_rating: Option<f64>,
    #[serde(alias = "ProductionYear", default)]
    production_year: Option<u32>,
    #[serde(alias = "Genres", default)]
    genres: Vec<String>,
    #[serde(alias = "ImageTags", rename(serialize = "image_tags"), default)]
    image_tags: ImageTags,
    #[serde(
        alias = "BackdropImageTags",
        rename(serialize = "backdrop_image_tags"),
        default
    )]
    backdrop_image_tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct ImageTags {
    #[serde(alias = "Primary", default)]
    primary: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ItemsPage {
    #[serde(alias = "Items", default)]
    items: Vec<Series>,
}

/// Shape consumed by the frontend tile renderer.
#[derive(Serialize, Debug)]
struct RandomShow {
    title: String,
    series_url: String,
    year: Option<u32>,
    rating: Option<f64>,
    genres: Vec<String>,
    overview: Option<String>,
    poster_url: Option<String>,
    backdrop_url: Option<String>,
}

fn image_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

fn to_show(base: &str, series: Series) -> RandomShow {
    let poster_url = series.image_tags.primary.as_ref().map(|_| {
        image_url(
            base,
            &format!(
                "/Items/{}/Images/Primary?maxWidth=328&quality=90",
                series.id
            ),
        )
    });
    let backdrop_url = if series.backdrop_image_tags.is_empty() {
        None
    } else {
        Some(image_url(
            base,
            &format!(
                "/Items/{}/Images/Backdrop/0?maxWidth=960&quality=70",
                series.id
            ),
        ))
    };
    RandomShow {
        title: series.name,
        series_url: image_url(base, &format!("/web/index.html#!/details?id={}", series.id)),
        year: series.production_year,
        rating: series
            .community_rating
            .map(|rating| (rating * 10.0).round() / 10.0),
        genres: series.genres,
        overview: series
            .overview
            .filter(|overview| !overview.trim().is_empty()),
        poster_url,
        backdrop_url,
    }
}

pub struct JellyfinRandomShowWidget {
    client: Client,
}

impl JellyfinRandomShowWidget {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Newer Jellyfin releases reject a bare `X-Api-Key`, so authenticate
    /// with the classic `Authorization: MediaBrowser Token="..."` header.
    fn authorized(&self, config: &Config, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!(
                "{}{}",
                config.url.trim_end_matches('/'),
                path
            ))
            .header(
                "Authorization",
                format!(
                    "MediaBrowser Token=\"{}\", Client=Vesta, Device=Vesta, DeviceId=Vesta, Version=1.0",
                    config.key
                ),
            )
    }

    /// Status-specific failures so the tile names what actually went wrong.
    fn api_error(status: StatusCode, context: &str) -> VestaError {
        let message = match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => format!(
                "Jellyfin rejected the API key (HTTP {status}): check the key on the server's dashboard."
            ),
            StatusCode::NOT_FOUND => {
                format!("The Jellyfin URL has no {context} route (HTTP 404): check the base URL.")
            }
            _ => format!("Jellyfin returned HTTP {status} while fetching {context}."),
        };
        VestaError::ApiError { status, message }
    }

    async fn check_reachable(
        &self,
        config: &Config,
        context: &str,
        request: reqwest::RequestBuilder,
    ) -> VestaResult<reqwest::Response> {
        let response = request.send().await.map_err(|error| VestaError::ApiError {
            status: StatusCode::BAD_GATEWAY,
            message: format!(
                "Could not reach the Jellyfin server at {} for {context}: {error}",
                config.url.trim_end_matches('/')
            ),
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::api_error(status, context));
        }
        Ok(response)
    }

    async fn fetch_random_series(&self, config: &Config) -> VestaResult<ItemsPage> {
        let mut params = vec![
            ("SortBy", "Random"),
            ("IncludeItemTypes", "Series"),
            ("Recursive", "true"),
            ("Limit", "1"),
            ("Fields", "Overview,Genres,CommunityRating,ProductionYear"),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ];
        if let Some(library) = config.library.as_deref().filter(|id| !id.is_empty()) {
            params.push(("ParentId", library));
        }

        let response = self
            .check_reachable(
                config,
                "the series list",
                self.authorized(config, "/Items").query(&params),
            )
            .await?;
        Ok(response.json::<ItemsPage>().await?)
    }

    async fn fetch_libraries(&self, config: &Config) -> VestaResult<Value> {
        let response = self
            .check_reachable(
                config,
                "the media libraries",
                self.authorized(config, "/Library/MediaFolders"),
            )
            .await?;
        Ok(response.json::<Value>().await?)
    }

    /// Options for the unsaved editor values; both fields must be filled.
    fn draft_config(config_json: &Value) -> VestaResult<Config> {
        serde_json::from_value(config_json.clone()).map_err(|_| {
            VestaError::InvalidWidgetConfig(
                "Enter the Jellyfin URL and API key to list the server's libraries.".to_string(),
            )
        })
    }
}

fn map_libraries(payload: &Value) -> Vec<DynamicOption> {
    let items = payload.get("Items").and_then(Value::as_array);
    items
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("Id")?.as_str()?;
                    let name = item.get("Name").and_then(Value::as_str)?;
                    (!id.is_empty() && !name.is_empty()).then(|| DynamicOption {
                        value: id.to_string(),
                        label: name.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[async_trait]
impl WidgetHandler for JellyfinRandomShowWidget {
    fn name(&self) -> &'static str {
        "JellyfinRandomShow"
    }

    fn description(&self) -> &'static str {
        "Shows a random series from your Jellyfin library."
    }

    fn default_size(&self) -> (u8, u8) {
        (4, 3)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 3)
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField {
                key: "url",
                label: "Jellyfin URL",
                kind: "url",
                required: true,
                description: "The base URL of your Jellyfin server.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "key",
                label: "API key",
                kind: "password",
                required: true,
                description: "Create one in the Jellyfin dashboard. Stored in config.toml; protect that file as a secret.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "library",
                label: "Library",
                kind: "remote_select",
                required: false,
                description: "Pick the media library the random series comes from. All libraries by default.",
                default: None,
                options: Vec::new(),
            },
        ]
    }

    type Config = Config;

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        let page = self.fetch_random_series(&config).await?;
        let base = config.url.trim_end_matches('/');
        match page.items.into_iter().next() {
            Some(series) => {
                let show = to_show(base, series);
                Ok(serde_json::to_value(&show)?)
            }
            None => Ok(json!({ "empty": true })),
        }
    }

    async fn field_options(
        &self,
        field: &str,
        config_json: &Value,
    ) -> VestaResult<Vec<DynamicOption>> {
        if field != "library" {
            return Err(VestaError::InvalidWidgetConfig(format!(
                "field '{field}' has no dynamic options"
            )));
        }
        let config = Self::draft_config(config_json)?;
        let payload = self.fetch_libraries(&config).await?;
        let options = map_libraries(&payload);
        if options.is_empty() {
            return Err(VestaError::ApiError {
                status: StatusCode::BAD_GATEWAY,
                message: "Jellyfin reported no media libraries for this API key.".to_string(),
            });
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::{ItemsPage, Series, to_show};
    use serde_json::json;

    #[test]
    fn map_libraries_reads_media_folders_shape() {
        let options = super::map_libraries(&json!({
            "Items": [
                { "Id": "lib1", "Name": "Anime", "CollectionType": "tvshows" },
                { "Id": "lib2", "Name": "Movies" },
                { "Name": "No id" },
                { "Id": "", "Name": "Empty id" }
            ]
        }));
        assert_eq!(
            options,
            vec![
                super::DynamicOption {
                    value: "lib1".into(),
                    label: "Anime".into()
                },
                super::DynamicOption {
                    value: "lib2".into(),
                    label: "Movies".into()
                },
            ]
        );
    }

    #[test]
    fn map_libraries_tolerates_unexpected_payload() {
        assert!(super::map_libraries(&json!({ "TotalRecordCount": 0 })).is_empty());
    }

    #[test]
    fn parses_the_real_pascal_case_payload() {
        let page: ItemsPage = serde_json::from_value(json!({
            "Items": [{
                "Id": "abc123",
                "Name": "Example Show",
                "Overview": "A test overview.",
                "CommunityRating": 8.66,
                "ProductionYear": 2020,
                "Genres": ["Drama"],
                "ImageTags": { "Primary": "tag1" },
                "BackdropImageTags": ["tag2"]
            }]
        }))
        .expect("real Jellyfin payloads use PascalCase keys");

        let series = &page.items[0];
        assert_eq!(series.id, "abc123");
        assert_eq!(series.name, "Example Show");
        assert_eq!(series.community_rating, Some(8.66));
        assert_eq!(series.production_year, Some(2020));
        assert_eq!(series.image_tags.primary.as_deref(), Some("tag1"));
    }

    #[test]
    fn random_show_maps_fields_and_image_urls() {
        let series = Series {
            id: "abc123".to_string(),
            name: "Example Show".to_string(),
            overview: Some("A test overview.".to_string()),
            community_rating: Some(8.66),
            production_year: Some(2020),
            genres: vec!["Drama".to_string()],
            image_tags: crate::widgets::jellyfin_random_show_widget::ImageTags {
                primary: Some("tag1".to_string()),
            },
            backdrop_image_tags: vec!["tag2".to_string()],
        };

        let show = to_show("https://jellyfin.example.com/", series);
        assert_eq!(show.title, "Example Show");
        assert_eq!(show.year, Some(2020));
        assert_eq!(show.rating, Some(8.7));
        assert_eq!(
            show.poster_url.as_deref(),
            Some(
                "https://jellyfin.example.com/Items/abc123/Images/Primary?maxWidth=328&quality=90"
            )
        );
        assert_eq!(
            show.backdrop_url.as_deref(),
            Some(
                "https://jellyfin.example.com/Items/abc123/Images/Backdrop/0?maxWidth=960&quality=70"
            )
        );
        assert!(
            show.series_url
                .contains("/web/index.html#!/details?id=abc123")
        );
    }

    #[test]
    fn random_show_omits_missing_images() {
        let series = Series {
            id: "abc123".to_string(),
            name: "No Images".to_string(),
            overview: Some(" ".to_string()),
            community_rating: None,
            production_year: None,
            genres: Vec::new(),
            image_tags: Default::default(),
            backdrop_image_tags: Vec::new(),
        };

        let show = to_show("https://jellyfin.example.com", series);
        assert!(show.poster_url.is_none());
        assert!(show.backdrop_url.is_none());
        assert!(show.overview.is_none());
        assert!(show.rating.is_none());
    }
}
