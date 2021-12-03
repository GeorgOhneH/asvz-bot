use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde::Serialize;
use tracing::{instrument, trace};
use url::Url;

use crate::asvz::error::AsvzError;
use reqwest_middleware::ClientWithMiddleware;

lazy_static! {
    static ref DUMMY_URL: Url = Url::parse("https://www.google.com/").unwrap();
    static ref BASE_AAI_URL: Url = Url::parse("https://aai-logon.ethz.ch").unwrap();
    static ref LOGIN_URL: Url = Url::parse("https://auth.asvz.ch/account/login").unwrap();
    static ref EXTERNAL_LOGIN_URL: Url =
        Url::parse("https://auth.asvz.ch/Account/ExternalLogin").unwrap();
    static ref AUTH_URL: Url =
        Url::parse("https://auth.asvz.ch/connect/authorize?client_id=55776bff-ef75-4c9d-9bdd-45e883ec38e0&redirect_uri=https://schalter.asvz.ch/tn/assets/oidc-login-redirect.html&response_type=id_token token&scope=openid profile tn-api tn-apiext tn-auth tn-hangfire").unwrap();

}

const LOCAL_STORAGE_FORM: [(&str, &str); 8] = [
    ("shib_idp_ls_exception.shib_idp_session_ss", ""),
    ("shib_idp_ls_success.shib_idp_session_ss", "false"),
    ("shib_idp_ls_value.shib_idp_session_ss", ""),
    ("shib_idp_ls_exception.shib_idp_persistent_ss", ""),
    ("shib_idp_ls_success.shib_idp_persistent_ss", "false"),
    ("shib_idp_ls_value.shib_idp_persistent_ss", ""),
    ("shib_idp_ls_supported", ""),
    ("_eventId_proceed", ""),
];

const AAI_FORM: [(&str, &str); 2] = [
    ("user_idp", "https://aai-logon.ethz.ch/idp/shibboleth"),
    ("Select", "Auswählen"),
];

fn unescape(str: &str) -> String {
    let mut r = String::new();
    html_escape::decode_html_entities_to_string(str, &mut r);
    r
}

#[instrument(skip(client, username, password))]
pub async fn asvz_login(
    client: &ClientWithMiddleware,
    username: &str,
    password: &str,
) -> Result<String, AsvzError> {
    lazy_static! {
        static ref VERIFI_TOKEN_RE: Regex =
            Regex::new("name=\"__RequestVerificationToken\".*value=\"(.+)\".*/").unwrap();
    }
    trace!("logging in");
    let login_text = client
        .get(LOGIN_URL.clone())
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    if !login_text.contains("action=\"/Account/Logout\"") {
        let verifi_token = &VERIFI_TOKEN_RE.captures(&login_text).unwrap()[1];

        let login_form = [
            ("provider", "SwitchAai"),
            ("__RequestVerificationToken", verifi_token),
        ];
        let response = client
            .post(EXTERNAL_LOGIN_URL.clone())
            .form(&login_form)
            .send()
            .await?
            .error_for_status()?;

        aai_login(
            client,
            username,
            password,
            response.url().clone(),
            &AAI_FORM,
        )
        .await?;
    }

    let mut auth_url: Url = AUTH_URL.clone();
    let nonce: String = std::iter::repeat_with(|| fastrand::digit(16))
        .take(32)
        .collect();
    let state: String = std::iter::repeat_with(|| fastrand::digit(16))
        .take(32)
        .collect();
    auth_url
        .query_pairs_mut()
        .append_pair("nonce", nonce.as_str())
        .append_pair("state", state.as_str());
    let response = client.get(auth_url).send().await?.error_for_status()?;

    let fragment = response
        .url()
        .fragment()
        .ok_or(AsvzError::UnexpectedFormat)?;
    let mut dummy_url = DUMMY_URL.clone();
    dummy_url.set_query(Some(fragment));
    let map = dummy_url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<HashMap<String, String>>();

    map.get("access_token")
        .map(Clone::clone)
        .ok_or(AsvzError::UnexpectedFormat)
}

async fn aai_login<T: Serialize + ?Sized>(
    client: &ClientWithMiddleware,
    username: &str,
    password: &str,
    url: Url,
    form: &T,
) -> Result<(), AsvzError> {
    lazy_static! {
        static ref ACTION_URL_RE: Regex =
            Regex::new("<form .*action=\"(.+)\" method=\"post\">").unwrap();
        static ref RELAY_STATE_RE: Regex =
            Regex::new("name=\"RelayState\" value=\"(.+)\"/>").unwrap();
        static ref SAMLRESPONSE_RE: Regex =
            Regex::new("name=\"SAMLResponse\" value=\"(.+)\"/").unwrap();
    }

    let text = client.post(url).form(form).send().await?.text().await?;
    let sam_text = if !text.contains("SAMLResponse") {
        let local_storage_part = &ACTION_URL_RE
            .captures(&text)
            .ok_or(AsvzError::UnexpectedFormat)?[1];
        let local_storage_url = BASE_AAI_URL.clone().join(local_storage_part)?;
        let login_page = client
            .post(local_storage_url)
            .form(&LOCAL_STORAGE_FORM)
            .send()
            .await?
            .text()
            .await?;

        let sso_form = [
            ("_eventId_proceed", ""),
            ("j_username", username),
            ("j_password", password),
        ];
        let sso_part = &ACTION_URL_RE
            .captures(&login_page)
            .ok_or(AsvzError::UnexpectedFormat)?[1];
        let sso_url = BASE_AAI_URL.clone().join(sso_part)?;
        client
            .post(sso_url)
            .form(&sso_form)
            .send()
            .await?
            .text()
            .await?
    } else {
        text
    };

    let sam_url = Url::parse(&unescape(
        &ACTION_URL_RE
            .captures(&sam_text)
            .ok_or(AsvzError::UnexpectedFormat)?[1],
    ))?;
    let ssm = unescape(
        &RELAY_STATE_RE
            .captures(&sam_text)
            .ok_or(AsvzError::UnexpectedFormat)?[1],
    );
    let sam = unescape(
        &SAMLRESPONSE_RE
            .captures(&sam_text)
            .ok_or(AsvzError::UnexpectedFormat)?[1],
    );

    let saml_form = [("RelayState", &ssm), ("SAMLResponse", &sam)];

    client
        .post(sam_url)
        .form(&saml_form)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}
