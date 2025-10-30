use {
    crate::{
        PROJECT_NAME, PROJECT_URL,
        error::{AppError, AppResult},
        hash::GlassVault,
        user::{Name, User, UserId},
    },
    anyhow::Result,
    axum_extra::extract::{CookieJar, cookie::Cookie},
    chrono::{DateTime, Duration, Utc},
    maud::{PreEscaped, html},
    serde::{Deserialize, Serialize},
    std::str::FromStr,
};

const SESSION_DURATION: Duration = Duration::days(30);
const COOKIE_NAME: &str = "auth";

#[derive(Serialize, Deserialize)]
struct Session {
    user_id: UserId,
    create_at: DateTime<Utc>,
}

pub(crate) fn create_auth_cookie(
    cookie_jar: CookieJar,
    user_id: UserId,
    remember: bool,
) -> Result<CookieJar> {
    let session = Session {
        user_id,
        create_at: Utc::now(),
    };

    let mut cookie = Cookie::new(COOKIE_NAME, GlassVault::new(session)?.to_string());

    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Strict);
    cookie.set_http_only(true);
    cookie.set_secure(!cfg!(debug_assertions));
    cookie.set_path("/");

    if remember {
        cookie.set_max_age(time::Duration::seconds(SESSION_DURATION.num_seconds()));
    }

    Ok(cookie_jar.add(cookie))
}

pub(crate) fn remove_auth_cookie(cookie_jar: CookieJar) -> CookieJar {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Strict);
    cookie.set_path("/");
    cookie.set_max_age(Some(time::Duration::seconds(0)));
    cookie_jar.add(cookie)
}

fn extract_session(cookie_jar: &CookieJar) -> Result<Option<Session>> {
    let cookie = match cookie_jar.get(COOKIE_NAME) {
        None => return Ok(None),
        Some(cookie) => cookie,
    };

    let session = GlassVault::<Session>::from_str(cookie.value())?.into_inner();

    if Utc::now() - session.create_at > SESSION_DURATION {
        tracing::warn!("auth session expired");
        return Ok(None);
    }

    Ok(Some(session))
}

pub(crate) async fn require_auth_user(cookie_jar: &CookieJar) -> AppResult<User> {
    let session = extract_session(cookie_jar)?.ok_or(AppError::Unauthorized)?;
    let user = User::by_id(session.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(user)
}

pub(crate) fn verification_email(name: &Name, link: &str) -> String {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Verify Your Email Address" }
                style {
                    (PreEscaped(r#"
                        body {
                            margin: 0;
                            padding: 0;
                            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
                            background-color: #f4f4f4;
                        }
                        .email-container {
                            max-width: 600px;
                            margin: 40px auto;
                            background-color: #ffffff;
                            border-radius: 8px;
                            overflow: hidden;
                            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
                        }
                        .header {
                            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                            padding: 40px 20px;
                            text-align: center;
                            color: #ffffff;
                        }
                        .header h1 {
                            margin: 0;
                            font-size: 28px;
                            font-weight: 600;
                        }
                        .content {
                            padding: 40px 30px;
                        }
                        .greeting {
                            font-size: 18px;
                            color: #333333;
                            margin-bottom: 20px;
                        }
                        .message {
                            font-size: 16px;
                            line-height: 1.6;
                            color: #555555;
                            margin-bottom: 30px;
                        }
                        .button-container {
                            text-align: center;
                            margin: 30px 0;
                        }
                        .verify-button {
                            display: inline-block;
                            padding: 14px 40px;
                            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                            color: #ffffff !important;
                            text-decoration: none;
                            border-radius: 6px;
                            font-weight: 600;
                            font-size: 16px;
                            transition: transform 0.2s;
                        }
                        .verify-button:hover {
                            transform: translateY(-2px);
                        }
                        .footer {
                            padding: 20px 30px;
                            background-color: #f8f9fa;
                            border-top: 1px solid #e9ecef;
                            font-size: 14px;
                            color: #6c757d;
                            line-height: 1.5;
                        }
                        .link-text {
                            font-size: 12px;
                            color: #6c757d;
                            word-break: break-all;
                            margin-top: 20px;
                        }
                        .security-note {
                            margin-top: 15px;
                            padding: 15px;
                            background-color: #fff3cd;
                            border-left: 4px solid #ffc107;
                            font-size: 14px;
                            color: #856404;
                        }
                        @media only screen and (max-width: 600px) {
                            body {
                                background-color: #ffffff !important;
                            }
                            .email-container {
                                margin: 0 !important;
                                border-radius: 0 !important;
                                box-shadow: none !important;
                            }
                            .content {
                                padding: 30px 20px !important;
                            }
                            .header {
                                padding: 30px 20px !important;
                            }
                        }
                    "#))
                }
            }
            body {
                div.email-container {
                    div.header {
                        h1 { "Email Verification" }
                    }
                    div.content {
                        p.greeting {
                            "Hey " (name) ","
                        }
                        p.message {
                            "Welcome to "
                            a href=(PROJECT_URL) { (PROJECT_NAME) }
                            ", my little experiment in training and running LLMs with Rust because... why not! "
                            "Before I allow you access you need to verify your email address. "
                            "Gotta make sure you're not a bot! 🤖"
                        }
                        p.message {
                            "Please click the button below to verify your email:"
                        }
                        div.button-container {
                            a.verify-button href=(link) {
                                "Verify Email Address"
                            }
                        }
                        div.security-note {
                            strong { "Security Note: " }
                            "If you didn't sign up, please ignore this email. "
                            "The verification link will expire shortly."
                        }
                        p.link-text {
                            "If the button doesn't work, copy and paste this link into your browser:"
                            br;
                            (link)
                        }
                    }
                    div.footer {
                        p {
                            "This is an automated message. You may however reply if you have any questions."
                        }
                    }
                }
            }
        }
    }.into_string()
}
