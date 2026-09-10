#[cfg(test)]
mod tests {
    use crate::handlers::user::{tests::get_first_users_id, CreateUserRequest};
    use crate::tests::helpers::tests::{assert_get, assert_post};

    const PATH: &str = "/api/v1/user";

    #[tokio::test]
    async fn it_gets_a_user() {
        let user_id = get_first_users_id();
        let url = format!("{}/{}", PATH, user_id);
        assert_get(&url).await;
    }

    #[tokio::test]
    async fn it_gets_all_users() {
        assert_get(PATH).await;
    }

    #[tokio::test]
    async fn it_creates_a_user() {
        let params = CreateUserRequest {
            first_name: "Satoshi".into(),
            last_name: "Nakamoto".into(),
            email: "satoshi@nakamotoinstitute.org".into(),
            password: "123456".into(),
        };
        assert_post(PATH, params).await;
    }
}
