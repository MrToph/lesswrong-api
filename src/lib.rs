use chrono::{DateTime, Utc};
use graphql_client::{GraphQLQuery, Response};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// we need to define the scalars used in our queries for derive(GraphQLQuery)
type Date = DateTime<Utc>;
#[allow(clippy::upper_case_acronyms)]
type JSON = serde_json::Value;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Server error: Status {0} - {1}")]
    ServerError(StatusCode, String),
    #[error("Post not found")]
    NotFound,
    #[error("Malformatted response: missing/malformatted field {0}")]
    MalformattedResponse(&'static str),
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, PartialEq)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub author: String,
    pub date: Date,
    pub content_html: String,
    pub content_markdown: String,
    pub slug: String,
    pub page_url: String,
    pub base_score: f64,
    pub word_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Comment {
    pub id: String,
    pub parent_comment_id: Option<String>,
    pub author: String,
    pub posted_at: chrono::DateTime<chrono::Utc>,
    pub page_url: String,
    pub base_score: f64,
    pub vote_count: f64,
    pub content_html: String,
    pub content_markdown: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.json",
    query_path = "graphql/post_query.graphql",
    response_derives = "Debug, Serialize, Deserialize"
)]
struct PostQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.json",
    query_path = "graphql/comments_query.graphql",
    response_derives = "Debug, Serialize, Deserialize"
)]
struct CommentsQuery;

pub struct LessWrongApiClient {
    client: reqwest::Client,
}

impl Default for LessWrongApiClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl LessWrongApiClient {
    pub async fn get_post(&self, post_id: &str) -> Result<Post, Error> {
        let variables = post_query::Variables {
            id: post_id.to_string(),
        };

        let response = self
            .client
            .post("https://www.lesswrong.com/graphql")
            .json(&PostQuery::build_query(variables))
            .send()
            .await?;
        let response: Response<post_query::ResponseData> = self.try_get_json(response).await?;

        let post_data = response
            .data
            .ok_or(Error::MalformattedResponse("data"))?
            .post
            .ok_or(Error::NotFound)?
            .result
            .ok_or(Error::MalformattedResponse("post"))?;

        let contents = post_data
            .contents
            .ok_or(Error::MalformattedResponse("post.contents"))?;

        let username = post_data.user.and_then(|u| u.display_name);

        Ok(Post {
            id: post_data.id.ok_or(Error::MalformattedResponse("post.id"))?,
            title: post_data
                .title
                .ok_or(Error::MalformattedResponse("post.title"))?,
            author: post_data
                .author
                .or(username)
                .ok_or(Error::MalformattedResponse("post.author"))?,
            date: post_data
                .posted_at
                .ok_or(Error::MalformattedResponse("post.posted_at"))?,
            slug: post_data
                .slug
                .ok_or(Error::MalformattedResponse("post.slug"))?,
            page_url: post_data.page_url,
            base_score: post_data
                .base_score
                .ok_or(Error::MalformattedResponse("post.base_score"))?,
            word_count: post_data
                .word_count
                .ok_or(Error::MalformattedResponse("post.word_count"))?,
            content_markdown: contents
                .markdown
                .ok_or(Error::MalformattedResponse("post.contents.markdown"))?,
            content_html: post_data
                .html_body
                .ok_or(Error::MalformattedResponse("post.html_body"))?,
        })
    }

    pub async fn get_comments(
        &self,
        post_id: &str,
        limit: i64,
    ) -> Result<HashMap<String, Comment>, Error> {
        let variables = comments_query::Variables {
            terms: Some(serde_json::json!({
                "view": "postCommentsTop",
                "postId": post_id,
                "limit": limit
            })),
        };

        let response = self
            .client
            .post("https://www.lesswrong.com/graphql")
            .json(&CommentsQuery::build_query(variables))
            .send()
            .await?;
        let response: Response<comments_query::ResponseData> = self.try_get_json(response).await?;

        let comments_data = response
            .data
            .ok_or(Error::MalformattedResponse("data"))?
            .comments
            .ok_or(Error::MalformattedResponse("comments"))?
            .results
            .ok_or(Error::MalformattedResponse("comments.results"))?;

        let comments = comments_data
            .into_iter()
            .flatten()
            // the .deleted field should always exist, default to filtering out weird comments
            // comments with a null htmlBody are either comments like:
            // "Note: this post originally appeared in a context without comments on Overcoming Bias" or
            // "[This comment is no longer endorsed by its author]"
            .filter(|c| !c.deleted.unwrap_or(false) && c.html_body.is_some() && !c.html_body.as_ref().unwrap().is_empty())
            .map(|c| {
                let page_url = c.page_url.expect("comment page_url should exist");

                let content_markdown = c
                    .contents
                    .as_ref()
                    .and_then(|ct| ct.markdown.as_ref())
                    .unwrap_or_else(|| {
                        panic!("comment contents markdown should exist for '{}'", &page_url)
                    });

                let username = c.user.and_then(|u| u.display_name);

                let comment = Comment {
                    id: c
                        .id
                        .unwrap_or_else(|| panic!("comment id should exist for '{}'", &page_url)),
                    parent_comment_id: c.parent_comment_id,
                    author: c.author.or(username).unwrap_or("anonymous".to_string()),
                    posted_at: c.posted_at.unwrap_or_else(|| {
                        panic!("comment posted_at should exist for '{}'", &page_url)
                    }),
                    base_score: c.base_score.unwrap_or_else(|| {
                        panic!("comment base_score should exist for '{}'", &page_url)
                    }),
                    vote_count: c.vote_count.unwrap_or_else(|| {
                        panic!("comment vote_count should exist for '{}'", &page_url)
                    }),
                    content_html: c.html_body.unwrap_or_else(|| {
                        panic!("comment html_body should exist for '{}'", &page_url)
                    }),
                    content_markdown: content_markdown.to_string(),
                    page_url,
                };
                (comment.id.clone(), comment)
            })
            .collect();

        Ok(comments)
    }

    async fn try_get_json<T>(&self, response: reqwest::Response) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        // Check for non-success status and print relevant information.
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("Error reading response text"));
            return Err(Error::ServerError(status, error_text));
        }

        match response.json::<T>().await {
            Ok(json) => Ok(json),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_post() {
        let api = LessWrongApiClient::default();
        let result = api.get_post("7ZqGiPHTpiDMwqMN2").await;

        let post = result.unwrap();
        assert_eq!(post.id, "7ZqGiPHTpiDMwqMN2");
        assert_eq!(post.title, "Twelve Virtues of Rationality");
        assert_eq!(post.author, "Eliezer Yudkowsky");
        assert_eq!(post.date.to_rfc3339(), "2006-01-01T08:00:05.370+00:00");
        assert_eq!(
            post.page_url,
            "https://www.lesswrong.com/posts/7ZqGiPHTpiDMwqMN2/twelve-virtues-of-rationality"
        );
        assert_eq!(post.slug, "twelve-virtues-of-rationality");
        assert!(post.base_score.is_finite());
        assert_eq!(post.word_count, 2228);
        assert_eq!(post.content_html.len(), 14134);
        assert_eq!(post.content_markdown.len(), 12964);
        println!("{:#?}", post);
    }

    #[tokio::test]
    async fn test_fail_get_not_found_post() {
        let api = LessWrongApiClient::default();
        let result = api.get_post("123456").await;
        let err = result.unwrap_err();
        match err {
            Error::NotFound => (),
            other => panic!("Expected NotFound error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_comments() {
        let api = LessWrongApiClient::default();
        let result = api.get_comments("7ZqGiPHTpiDMwqMN2", 9999).await;
        let comments = if let Ok(comments) = result {
            comments
        } else {
            panic!("Failed to fetch comments: {}", result.unwrap_err());
        };

        assert!(!comments.is_empty(), "Should return non-empty comments");

        // Verify parent comment relationships
        let has_replies = comments.values().any(|c| c.parent_comment_id.is_some());
        assert!(has_replies, "Should contain comment threads");
    }
}
