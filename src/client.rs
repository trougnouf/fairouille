use crate::model::{CalendarListEntry, Task};
use libdav::CalDavClient;
use libdav::PropertyName;
use libdav::dav::WebDavClient;

use http::Uri;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::sync::Arc;
use tower_http::auth::AddAuthorization;

type HttpsClient = AddAuthorization<
    Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        String,
    >,
>;

#[derive(Clone, Debug)]
pub struct RustyClient {
    client: CalDavClient<HttpsClient>,
    calendar_url: Option<String>,
}

impl RustyClient {
    pub fn new(url: &str, user: &str, pass: &str) -> Result<Self, String> {
        let uri: Uri = url
            .parse()
            .map_err(|e: http::uri::InvalidUri| e.to_string())?;
        let tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();
        let https_connector = HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_or_http()
            .enable_http1()
            .build();
        let http_client = Client::builder(TokioExecutor::new()).build(https_connector);
        let auth_client = AddAuthorization::basic(http_client, user, pass);
        let webdav = WebDavClient::new(uri, auth_client);
        Ok(Self {
            client: CalDavClient::new(webdav),
            calendar_url: None,
        })
    }

    pub async fn discover_calendar(&mut self) -> Result<String, String> {
        let base_path = self.client.base_url().path().to_string();
        if let Ok(resources) = self.client.list_resources(&base_path).await {
            if resources.iter().any(|r| r.href.ends_with(".ics")) {
                self.calendar_url = Some(base_path.clone());
                return Ok(base_path);
            }
        }
        match self.client.find_current_user_principal().await {
            Ok(Some(principal)) => {
                if let Ok(homes) = self.client.find_calendar_home_set(&principal).await {
                    if let Some(home_url) = homes.first() {
                        if let Ok(cals) = self.client.find_calendars(home_url).await {
                            if let Some(first) = cals.first() {
                                self.calendar_url = Some(first.href.clone());
                                return Ok(first.href.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        self.calendar_url = Some(base_path.clone());
        Ok(base_path)
    }

    pub async fn get_calendars(&self) -> Result<Vec<CalendarListEntry>, String> {
        let principal = self
            .client
            .find_current_user_principal()
            .await
            .map_err(|e| format!("{:?}", e))?
            .ok_or("No principal")?;
        let homes = self
            .client
            .find_calendar_home_set(&principal)
            .await
            .map_err(|e| format!("{:?}", e))?;
        let home_url = homes.first().ok_or("No home set")?;
        let collections = self
            .client
            .find_calendars(home_url)
            .await
            .map_err(|e| format!("{:?}", e))?;
        let mut calendars = Vec::new();
        for col in collections {
            let prop = PropertyName::new("DAV:", "displayname");
            let name = self
                .client
                .get_property(&col.href, &prop)
                .await
                .unwrap_or(None)
                .unwrap_or(col.href.clone());
            calendars.push(CalendarListEntry {
                name,
                href: col.href,
                color: None,
            });
        }
        Ok(calendars)
    }

    pub fn set_calendar(&mut self, href: &str) {
        self.calendar_url = Some(href.to_string());
    }

    pub async fn get_tasks(&self) -> Result<Vec<Task>, String> {
        let cal_url = self.calendar_url.as_ref().ok_or("No calendar")?;
        let resources = self
            .client
            .list_resources(cal_url)
            .await
            .map_err(|e| format!("{:?}", e))?;
        let hrefs: Vec<String> = resources
            .iter()
            .map(|r| r.href.clone())
            .filter(|h| h.ends_with(".ics"))
            .collect();
        if hrefs.is_empty() {
            return Ok(vec![]);
        }
        let fetched = self
            .client
            .get_calendar_resources(cal_url, &hrefs)
            .await
            .map_err(|e| format!("{:?}", e))?;
        let mut tasks = Vec::new();
        for item in fetched {
            if let Ok(content) = item.content {
                if !content.data.is_empty() {
                    if let Ok(task) = Task::from_ics(&content.data, content.etag, item.href) {
                        tasks.push(task);
                    }
                }
            }
        }
        Ok(tasks)
    }

    pub async fn update_task(&self, task: &mut Task) -> Result<(), String> {
        let bytes = task.to_ics().as_bytes().to_vec();
        let res = self
            .client
            .update_resource(
                &task.href,
                bytes,
                &task.etag,
                b"text/calendar; charset=utf-8; component=VTODO",
            )
            .await
            .map_err(|e| format!("{:?}", e))?;
        if let Some(new_etag) = res {
            task.etag = new_etag;
        }
        Ok(())
    }

    pub async fn create_task(&self, task: &mut Task) -> Result<(), String> {
        let cal_url = self.calendar_url.as_ref().ok_or("No calendar")?;
        let filename = format!("{}.ics", task.uid);
        let full_href = if cal_url.ends_with('/') {
            format!("{}{}", cal_url, filename)
        } else {
            format!("{}/{}", cal_url, filename)
        };
        task.href = full_href.clone();
        let bytes = task.to_ics().as_bytes().to_vec();
        let res = self
            .client
            .create_resource(&full_href, bytes, b"text/calendar")
            .await
            .map_err(|e| format!("{:?}", e))?;
        if let Some(new_etag) = res {
            task.etag = new_etag;
        }
        Ok(())
    }

    pub async fn delete_task(&self, task: &Task) -> Result<(), String> {
        self.client
            .delete(&task.href, &task.etag)
            .await
            .map_err(|e| format!("{:?}", e))?;
        Ok(())
    }

    // --- CORE LOGIC: Toggle & Respawn ---
    pub async fn toggle_task(&self, task: &mut Task) -> Result<(Task, Option<Task>), String> {
        task.completed = !task.completed;
        let next_task = if task.completed { task.respawn() } else { None };

        let mut created_task = None;
        if let Some(mut next) = next_task {
            self.create_task(&mut next).await?;
            created_task = Some(next);
        }
        self.update_task(task).await?;
        Ok((task.clone(), created_task))
    }
}

#[derive(Debug)]
struct NoVerifier;
impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        use rustls::SignatureScheme::*;
        vec![
            RSA_PKCS1_SHA256,
            RSA_PKCS1_SHA384,
            RSA_PKCS1_SHA512,
            ECDSA_NISTP256_SHA256,
            RSA_PSS_SHA256,
            ED25519,
        ]
    }
}
