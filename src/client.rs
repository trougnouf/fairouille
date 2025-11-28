use crate::model::{CalendarListEntry, Task};
use libdav::CalDavClient;
use libdav::PropertyName;
use libdav::dav::WebDavClient;
use rustls_native_certs;

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
    // Removed stateful 'calendar_url'
}

impl RustyClient {
    pub fn new(url: &str, user: &str, pass: &str, insecure: bool) -> Result<Self, String> {
        let uri: Uri = url
            .parse()
            .map_err(|e: http::uri::InvalidUri| e.to_string())?;

        let https_connector = if insecure {
            // INSECURE PATH
            let tls_config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoVerifier))
                .with_no_client_auth();

            HttpsConnectorBuilder::new()
                .with_tls_config(tls_config)
                .https_or_http()
                .enable_http1()
                .build()
        } else {
            // SECURE PATH (Final Corrected Version for v0.8+)
            let mut root_store = rustls::RootCertStore::empty();

            // This function now returns a struct, not a Result.
            let result = rustls_native_certs::load_native_certs();

            // The `result.errors` vector contains any non-fatal errors.
            // We add all the certificates that were successfully loaded.
            root_store.add_parsable_certificates(result.certs);

            if root_store.is_empty() {
                // This is the true failure condition: not a single valid
                // certificate could be loaded from the system.
                return Err(
                "No valid system certificates found. Cannot establish secure connection. Consider using 'allow_insecure_certs'."
                .to_string()
            );
            }

            let tls_config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            HttpsConnectorBuilder::new()
                .with_tls_config(tls_config)
                .https_or_http()
                .enable_http1()
                .build()
        };
        // -------------------------------------------------------------------

        let http_client = Client::builder(TokioExecutor::new()).build(https_connector);
        let auth_client = AddAuthorization::basic(http_client, user, pass);
        let webdav = WebDavClient::new(uri, auth_client);
        Ok(Self {
            client: CalDavClient::new(webdav),
        })
    }

    pub async fn discover_calendar(&self) -> Result<String, String> {
        let base_path = self.client.base_url().path().to_string();

        // 1. Try directly if it looks like a calendar (resource list)
        if let Ok(resources) = self.client.list_resources(&base_path).await
            && resources.iter().any(|r| r.href.ends_with(".ics"))
        {
            return Ok(base_path);
        }

        // 2. Try Principal -> Home Set -> First Calendar
        if let Ok(Some(principal)) = self.client.find_current_user_principal().await
            && let Ok(homes) = self.client.find_calendar_home_set(&principal).await
            && let Some(home_url) = homes.first()
            && let Ok(cals) = self.client.find_calendars(home_url).await
            && let Some(first) = cals.first()
        {
            return Ok(first.href.clone());
        }

        // Fallback to base
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
            // NOTE: We attempted to filter by 'supported-calendar-component-set' here,
            // but the underlying XML parser discards attributes on empty tags (e.g. <comp name="VTODO"/>),
            // making it impossible to distinguish between VEVENT-only and VTODO-only calendars
            // without a raw PROPFIND, which libdav hides.
            // We default to showing all calendars.

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

    pub async fn get_tasks(&self, calendar_href: &str) -> Result<Vec<Task>, String> {
        let resources = self
            .client
            .list_resources(calendar_href)
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
            .get_calendar_resources(calendar_href, &hrefs)
            .await
            .map_err(|e| format!("{:?}", e))?;
        let mut tasks = Vec::new();
        for item in fetched {
            if let Ok(content) = item.content
                && !content.data.is_empty()
            {
                // Pass calendar_href so the task knows its parent
                if let Ok(task) = Task::from_ics(
                    &content.data,
                    content.etag,
                    item.href,
                    calendar_href.to_string(),
                ) {
                    tasks.push(task);
                }
            }
        }
        Ok(tasks)
    }

    // Fetch ALL tasks from ALL calendars concurrently
    pub async fn get_all_tasks(
        &self,
        calendars: &[CalendarListEntry],
    ) -> Result<Vec<(String, Vec<Task>)>, String> {
        let mut handles = Vec::new();

        for cal in calendars {
            let client = self.clone();
            let href = cal.href.clone();
            handles.push(tokio::spawn(async move {
                let tasks = client.get_tasks(&href).await;
                (href, tasks)
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok((href, task_res)) = handle.await
                && let Ok(tasks) = task_res
            {
                results.push((href, tasks));
            }
        }
        Ok(results)
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
        if task.calendar_href.is_empty() {
            return Err("Task has no calendar assigned".to_string());
        }
        let cal_url = &task.calendar_href;

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

    pub async fn toggle_task(&self, task: &mut Task) -> Result<(Task, Option<Task>), String> {
        if task.status == crate::model::TaskStatus::Completed {
            task.status = crate::model::TaskStatus::NeedsAction;
        } else {
            task.status = crate::model::TaskStatus::Completed;
        }

        let next_task = if task.status == crate::model::TaskStatus::Completed {
            task.respawn()
        } else {
            None
        };
        let mut created_task = None;
        if let Some(mut next) = next_task {
            // Next task inherits calendar_href from the parent in respawn() (via clone)
            self.create_task(&mut next).await?;
            created_task = Some(next);
        }
        self.update_task(task).await?;
        Ok((task.clone(), created_task))
    }
}

// Reuse the existing verifier
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
