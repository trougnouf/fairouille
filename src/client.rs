use crate::model::{CalendarListEntry, Task};
use libdav::CalDavClient;
use libdav::PropertyName;
use libdav::dav::WebDavClient; // <--- FIX: Import from root, not ::dav::

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
        let client = CalDavClient::new(webdav);

        Ok(Self {
            client,
            calendar_url: None,
        })
    }

    pub async fn discover_calendar(&mut self) -> Result<String, String> {
        let base_path = self.client.base_url().path().to_string();

        // Check provided URL first
        if let Ok(resources) = self.client.list_resources(&base_path).await {
            let has_ics = resources.iter().any(|r| r.href.ends_with(".ics"));
            if has_ics {
                self.calendar_url = Some(base_path.clone());
                return Ok(base_path);
            }
        }

        // Discovery Fallback
        match self.client.find_current_user_principal().await {
            Ok(Some(principal)) => {
                if let Ok(homes) = self.client.find_calendar_home_set(&principal).await {
                    if let Some(home_url) = homes.first() {
                        if let Ok(calendars) = self.client.find_calendars(home_url).await {
                            if let Some(first_cal) = calendars.first() {
                                let href = first_cal.href.clone();
                                self.calendar_url = Some(href.clone());
                                return Ok(href);
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

    // NEW: Fetch list of calendars
    pub async fn get_calendars(&self) -> Result<Vec<CalendarListEntry>, String> {
        // 1. Find the Home Set (where calendars live)
        let principal = self
            .client
            .find_current_user_principal()
            .await
            .map_err(|e| format!("Principal error: {:?}", e))?
            .ok_or("No principal")?;

        let homes = self
            .client
            .find_calendar_home_set(&principal)
            .await
            .map_err(|e| format!("Home error: {:?}", e))?;
        let home_url = homes.first().ok_or("No home set")?;

        // 2. Find actual collections
        let collections = self
            .client
            .find_calendars(home_url)
            .await
            .map_err(|e| format!("Find calendars error: {:?}", e))?;

        let mut calendars = Vec::new();

        // 3. Fetch "displayname" for each calendar
        for col in collections {
            let prop_name = PropertyName::new("DAV:", "displayname");

            let name = match self.client.get_property(&col.href, &prop_name).await {
                Ok(Some(n)) => n,
                _ => col.href.clone(),
            };

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
        let cal_url = self.calendar_url.as_ref().ok_or("No calendar discovered")?;

        let resources = self
            .client
            .list_resources(cal_url)
            .await
            .map_err(|e| format!("Failed to list resources: {:?}", e))?;

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
            .map_err(|e| format!("Fetch error: {:?}", e))?;

        let mut tasks = Vec::new();

        for item in fetched {
            if let Ok(content) = item.content {
                let body = content.data;
                let etag = content.etag;

                if body.is_empty() {
                    continue;
                }

                if let Ok(task) = Task::from_ics(&body, etag, item.href) {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    pub async fn update_task(&self, task: &mut Task) -> Result<(), String> {
        let ics_body = task.to_ics();
        let bytes = ics_body.as_bytes().to_vec();

        let result = self
            .client
            .update_resource(
                &task.href,
                bytes,
                &task.etag,
                b"text/calendar; charset=utf-8; component=VTODO",
            )
            .await
            .map_err(|e| format!("Update failed: {:?}", e))?;

        if let Some(new_etag) = result {
            task.etag = new_etag;
        }

        Ok(())
    }

    pub async fn create_task(&self, task: &mut Task) -> Result<(), String> {
        let cal_url = self.calendar_url.as_ref().ok_or("No calendar")?;

        let filename = format!("{}.ics", task.uid);

        // Construct URL (Naive join)
        let full_href = if cal_url.ends_with('/') {
            format!("{}{}", cal_url, filename)
        } else {
            format!("{}/{}", cal_url, filename)
        };

        task.href = full_href.clone();

        let ics_body = task.to_ics();
        let bytes = ics_body.as_bytes().to_vec();

        let result = self
            .client
            .create_resource(&full_href, bytes, b"text/calendar")
            .await
            .map_err(|e| format!("Create failed: {:?}", e))?;

        if let Some(new_etag) = result {
            task.etag = new_etag;
        }

        Ok(())
    }

    pub async fn delete_task(&self, task: &Task) -> Result<(), String> {
        self.client
            .delete(&task.href, &task.etag)
            .await
            .map_err(|e| format!("Delete failed: {:?}", e))?;
        Ok(())
    }
}

// --- VERIFIER ---
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
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
