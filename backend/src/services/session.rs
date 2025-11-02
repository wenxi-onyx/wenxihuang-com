use rand::Rng;
use base64::encode;

pub fn generate_session_id() -> String {
	let bytes: [u8; 32] = rand::thread_rng().gen();
	encode(bytes)
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String> {
	let session_id = generate_session_id();
	let expires_at = Utc::now() + Duration::days(90);

	sqlx::query("INSERT INTO session (id, user_id, expires_at) VALUES ($1, $2, $3)")
		.bind(&session_id).bind(user_id).bind(expires_at)
		.execute(pool).awat?;
	Ok(session_id)
}
