#[cfg(test)]
mod tests {
    use crate::tests::helpers::tests::assert_get;

    #[tokio::test]
    async fn test_health() {
        assert_get("/health").await;
    }
}
