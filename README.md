# lesswrong-api

A simple API crate to fetch a [lesswrong](https://www.lesswrong.com/) post and its comments by ID.

## Development

```bash
cargo add --git https://github.com/MrToph/lesswrong-api.git
```

```rust
let id = "46qnWRSR7L2eyNbMA";
let client = LessWrongApiClient::default();
let post = client.get_post(id).await?;
let comments = client.get_comments(id, 9999).await?;
```

## Resources

This uses the official [LessWrong GraphQL API](https://www.lesswrong.com/graphiql?query=%0A%20%20%20%20%7B%0A%20%20%20%20%20%20comments%28input%3A%20%7B%0A%20%20%20%20%20%20%20%20terms%3A%20%7B%0A%20%20%20%20%20%20%20%20%20%20view%3A%20%22postCommentsTop%22%2C%0A%20%20%20%20%20%20%20%20%20%20postId%3A%20%22ZTcNDnz2xrhpL2cpc%22%2C%0A%20%20%20%20%20%20%20%20%7D%0A%20%20%20%20%20%20%7D%29%20%7B%0A%20%20%20%20%20%20%20%20results%20%7B%0A%20%20%20%20%20%20%20%20%20%20_id%0A%20%20%20%20%20%20%20%20%20%20user%20%7B%0A%20%20%20%20%20%20%20%20%20%20%20%20_id%0A%20%20%20%20%20%20%20%20%20%20%20%20username%0A%20%20%20%20%20%20%20%20%20%20%20%20displayName%0A%20%20%20%20%20%20%20%20%20%20%20%20slug%0A%20%20%20%20%20%20%20%20%20%20%20%20bio%0A%20%20%20%20%20%20%20%20%20%20%7D%0A%20%20%20%20%20%20%20%20%20%20userId%0A%20%20%20%20%20%20%20%20%20%20author%0A%20%20%20%20%20%20%20%20%20%20parentCommentId%0A%20%20%20%20%20%20%20%20%20%20pageUrl%0A%20%20%20%20%20%20%20%20%20%20htmlBody%0A%20%20%20%20%20%20%20%20%20%20baseScore%0A%20%20%20%20%20%20%20%20%20%20voteCount%0A%20%20%20%20%20%20%20%20%20%20postedAt%0A%20%20%20%20%20%20%20%20%7D%0A%20%20%20%20%20%20%7D%0A%20%20%20%20%7D%0A%20%20%20%20) under the hood.

The entire schema was dumped through an introspection query and can be seen in [`graphql/schema.json`](./graphql/schema.json):

```bash
graphql-client introspect-schema https://www.lesswrong.com/graphql > graphql/schema.json
```

The alternative [lesswrong frontend](https://lw2.issarice.com/posts/7ZqGiPHTpiDMwqMN2/understanding-benchmarks-and-motivating-evaluations?format=queries) can be studied to see some example queries.
