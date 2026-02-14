use serde::Serialize;

#[derive(Serialize, serde::Deserialize, Clone)]
pub struct FrontendRequest<T: Send + Serialize + Clone + 'static> {
    pub id: String,
    pub data: T,
}
