async fn login(
    State(pool): State<PgPool>,
    cookies: Cookies,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let user = User::find_by_email(&pool, &req.email).await?;
    verify_password(&req.password, &user.password_hash)?;

    let session_id = create_session(&pool, user.id).await?;

    let mut cookie = Cookie::new("session_id", session_id);
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Duration::days(30));
    cookies.add(cookie);

    Ok(Json(AuthResponse { user }))
}
