use crate::cache::Cache;
use crate::config::Config;
use crate::journal::{Action, Journal};
use crate::model::{CalendarListEntry, Task, TaskStatus};
use crate::storage::{LOCAL_CALENDAR_HREF, LocalStorage};
use libdav::CalDavClient;
use libdav::dav::WebDavClient;

use http::Uri;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use rustls_native_certs;
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
    // Wrapped in Option to support Offline-only mode
    client: Option<CalDavClient<HttpsClient>>,
}

impl RustyClient {
    pub fn new(url: &str, user: &str, pass: &str, insecure: bool) -> Result<Self, String> {
        if url.is_empty() {
            return Ok(Self { client: None }); // Offline Mode
        }

        let uri: Uri = url
            .parse()
            .map_err(|e: http::uri::InvalidUri| e.to_string())?;

        let https_connector = if insecure {
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
            let mut root_store = rustls::RootCertStore::empty();
            let result = rustls_native_certs::load_native_certs();
            root_store.add_parsable_certificates(result.certs);

            if root_store.is_empty() {
                return Err("No valid system certificates found.".to_string());
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

        let http_client = Client::builder(TokioExecutor::new()).build(https_connector);
        let auth_client = AddAuthorization::basic(http_client, user, pass);
        let webdav = WebDavClient::new(uri, auth_client);
        Ok(Self {
            client: Some(CalDavClient::new(webdav)),
        })
    }

    pub async fn discover_calendar(&self) -> Result<String, String> {
        if let Some(client) = &self.client {
            let base_path = client.base_url().path().to_string();

            // 1. Try directly if it looks like a calendar (resource list)
            if let Ok(resources) = client.list_resources(&base_path).await
                && resources.iter().any(|r| r.href.ends_with(".ics"))
            {
                return Ok(base_path);
            }

            // 2. Try Principal -> Home Set -> First Calendar
            if let Ok(Some(principal)) = client.find_current_user_principal().await
                && let Ok(homes) = client.find_calendar_home_set(&principal).await
                && let Some(home_url) = homes.first()
                && let Ok(cals) = client.find_calendars(home_url).await
                && let Some(first) = cals.first()
            {
                return Ok(first.href.clone());
            }

            // Fallback to base
            Ok(base_path)
        } else {
            Err("Offline".to_string())
        }
    }

    // --- SHARED INIT LOGIC (GUI & TUI) ---
    // This handles: Connection -> Journal Sync -> Calendar Fetch (w/ Cache fallback) -> Task Fetch
    pub async fn connect_with_fallback(
        config: Config,
    ) -> Result<
        (
            Self,                   // Client
            Vec<CalendarListEntry>, // Calendars
            Vec<Task>,              // Tasks (active cal)
            Option<String>,         // Active Href
            Option<String>,         // Warning/Error Message
        ),
        String,
    > {
        let client = Self::new(
            &config.url,
            &config.username,
            &config.password,
            config.allow_insecure_certs,
        )
        .map_err(|e| e.to_string())?;

        // 1. Flush Journal (Attempt)
        let _ = client.sync_journal().await;

        // 2. Fetch Calendars with Fallback
        let (calendars, warning) = match client.get_calendars().await {
            Ok(c) => {
                // Success: Save to cache
                let _ = Cache::save_calendars(&c);
                (c, None)
            }
            Err(e) => {
                let err_str = e.to_string();
                // Fatal cert error? Fail hard.
                if err_str.contains("InvalidCertificate") {
                    return Err(format!("Connection failed: {}", err_str));
                }
                // Otherwise (Timeout/DNS/Auth), load from cache
                let cached = Cache::load_calendars().unwrap_or_default();
                (
                    cached,
                    Some("Offline Mode (Network unreachable)".to_string()),
                )
            }
        };

        // 3. Determine Active
        let mut active_href = None;
        if let Some(def_cal) = &config.default_calendar
            && let Some(found) = calendars
                .iter()
                .find(|c| c.name == *def_cal || c.href == *def_cal)
        {
            active_href = Some(found.href.clone());
        }

        // Only try discovery if we are online and explicit default failed
        if active_href.is_none()
            && warning.is_none()
            && let Ok(href) = client.discover_calendar().await
        {
            active_href = Some(href);
        }

        // 4. Fetch Tasks (only if online)
        // If offline, we return empty list here. The UI (GUI/TUI) is responsible
        // for calling Cache::load() for the active calendar.
        let tasks = if warning.is_none() {
            if let Some(ref h) = active_href {
                client.get_tasks(h).await.unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        Ok((client, calendars, tasks, active_href, warning))
    }

    // --- READ OPERATIONS ---

    pub async fn get_calendars(&self) -> Result<Vec<CalendarListEntry>, String> {
        // If we have a network client, fetch from network
        if let Some(client) = &self.client {
            // ... (Copy your existing get_calendars logic here) ...
            // For brevity in this snippet, assumes implementation exists.
            // Be sure to replace `self.client` with `client` inside the block.
            let principal = client
                .find_current_user_principal()
                .await
                .map_err(|e| format!("{:?}", e))?
                .ok_or("No principal")?;
            let homes = client
                .find_calendar_home_set(&principal)
                .await
                .map_err(|e| format!("{:?}", e))?;
            let home_url = homes.first().ok_or("No home set")?;
            let collections = client
                .find_calendars(home_url)
                .await
                .map_err(|e| format!("{:?}", e))?;

            let mut calendars = Vec::new();
            for col in collections {
                let prop = libdav::PropertyName::new("DAV:", "displayname");
                let name = client
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
        } else {
            // Offline mode: return empty list (Local is injected by UI/Store)
            Ok(vec![])
        }
    }

    pub async fn get_tasks(&self, calendar_href: &str) -> Result<Vec<Task>, String> {
        // >>> ROUTING <<<
        if calendar_href == LOCAL_CALENDAR_HREF {
            return LocalStorage::load().map_err(|e| e.to_string());
        }

        if let Some(client) = &self.client {
            // [SYNC JOURNAL]: Before fetching fresh data, try to push pending changes
            // so we don't overwrite our own offline edits with old server data.
            // We ignore errors here (if sync fails, we still want to try to read).
            let _ = self.sync_journal().await;
            // ... (Copy existing get_tasks logic) ...
            let resources = client
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
            let fetched = client
                .get_calendar_resources(calendar_href, &hrefs)
                .await
                .map_err(|e| format!("{:?}", e))?;
            let mut tasks = Vec::new();
            for item in fetched {
                if let Ok(content) = item.content
                    && !content.data.is_empty()
                    && let Ok(task) = Task::from_ics(
                        &content.data,
                        content.etag,
                        item.href,
                        calendar_href.to_string(),
                    )
                {
                    tasks.push(task);
                }
            }
            Ok(tasks)
        } else {
            Err("Offline: Cannot fetch remote calendar".to_string())
        }
    }

    // --- WRITE OPERATIONS (ROUTED) ---

    pub async fn create_task(&self, task: &mut Task) -> Result<(), String> {
        if task.calendar_href == LOCAL_CALENDAR_HREF {
            let mut all = LocalStorage::load().unwrap_or_default();
            all.push(task.clone());
            LocalStorage::save(&all).map_err(|e| e.to_string())?;
            return Ok(());
        }

        if let Some(client) = &self.client {
            let filename = format!("{}.ics", task.uid);
            let full_href = if task.calendar_href.ends_with('/') {
                format!("{}{}", task.calendar_href, filename)
            } else {
                format!("{}/{}", task.calendar_href, filename)
            };
            task.href = full_href.clone();
            let bytes = task.to_ics().as_bytes().to_vec();

            // Attempt Network Call
            match client
                .create_resource(&full_href, bytes, b"text/calendar")
                .await
            {
                Ok(res) => {
                    if let Some(new_etag) = res {
                        task.etag = new_etag;
                    }
                    Ok(())
                }
                Err(e) => {
                    // Network failed. Queue it.
                    eprintln!(
                        "Network error during Create. Queuing offline. Error: {:?}",
                        e
                    );
                    Journal::push(Action::Create(task.clone())).map_err(|je| je.to_string())?;
                    Ok(()) // Return Ok to UI (Optimistic)
                }
            }
        } else {
            Err("Offline".to_string())
        }
    }

    pub async fn update_task(&self, task: &mut Task) -> Result<(), String> {
        if task.calendar_href == LOCAL_CALENDAR_HREF {
            let mut all = LocalStorage::load().unwrap_or_default();
            if let Some(idx) = all.iter().position(|t| t.uid == task.uid) {
                all[idx] = task.clone();
                LocalStorage::save(&all).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        if let Some(client) = &self.client {
            let bytes = task.to_ics().as_bytes().to_vec();

            match client
                .update_resource(
                    &task.href,
                    bytes,
                    &task.etag,
                    b"text/calendar; charset=utf-8; component=VTODO",
                )
                .await
            {
                Ok(res) => {
                    if let Some(new_etag) = res {
                        task.etag = new_etag;
                    }
                    Ok(())
                }
                Err(e) => {
                    // Network failed. Queue it.
                    eprintln!(
                        "Network error during Update. Queuing offline. Error: {:?}",
                        e
                    );
                    Journal::push(Action::Update(task.clone())).map_err(|je| je.to_string())?;
                    Ok(())
                }
            }
        } else {
            Err("Offline".to_string())
        }
    }

    pub async fn delete_task(&self, task: &Task) -> Result<(), String> {
        if task.calendar_href == LOCAL_CALENDAR_HREF {
            let mut all = LocalStorage::load().unwrap_or_default();
            all.retain(|t| t.uid != task.uid);
            LocalStorage::save(&all).map_err(|e| e.to_string())?;
            return Ok(());
        }

        if let Some(client) = &self.client {
            match client.delete(&task.href, &task.etag).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    // Network failed. Queue it.
                    eprintln!(
                        "Network error during Delete. Queuing offline. Error: {:?}",
                        e
                    );
                    Journal::push(Action::Delete(task.clone())).map_err(|je| je.to_string())?;
                    Ok(())
                }
            }
        } else {
            Err("Offline".to_string())
        }
    }

    pub async fn toggle_task(&self, task: &mut Task) -> Result<(Task, Option<Task>), String> {
        if task.status == TaskStatus::Completed {
            task.status = TaskStatus::NeedsAction;
        } else {
            task.status = TaskStatus::Completed;
        }

        let next_task = if task.status == TaskStatus::Completed {
            task.respawn()
        } else {
            None
        };

        if task.calendar_href == LOCAL_CALENDAR_HREF {
            // ... (existing local logic) ...
            let mut all = LocalStorage::load().unwrap_or_default();
            if let Some(idx) = all.iter().position(|t| t.uid == task.uid) {
                all[idx] = task.clone();
            }
            if let Some(new_t) = &next_task {
                all.push(new_t.clone());
            }
            LocalStorage::save(&all).map_err(|e| e.to_string())?;
            return Ok((task.clone(), next_task));
        }

        // Network Transaction (Resilient via create/update_task)
        let mut created_task = None;
        if let Some(mut next) = next_task {
            self.create_task(&mut next).await?;
            created_task = Some(next);
        }
        self.update_task(task).await?;
        Ok((task.clone(), created_task))
    }

    pub async fn get_all_tasks(
        &self,
        calendars: &[CalendarListEntry],
    ) -> Result<Vec<(String, Vec<Task>)>, String> {
        let mut handles = Vec::new();
        // We clone self to pass into threads.
        // Note: 'client' inside is Arc-like? No, CalDavClient is struct.
        // We might need to ensure RustyClient is cheap to clone.
        // CalDavClient<HttpsClient> uses Hyper Client which is cheap to clone.

        for cal in calendars {
            let client = self.clone();
            let href = cal.href.clone();
            handles.push(tokio::spawn(async move {
                let tasks = client.get_tasks(&href).await;
                (href, tasks)
            }));
        }
        // ... (collect results logic) ...
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

    pub async fn move_task(&self, task: &Task, new_calendar_href: &str) -> Result<Task, String> {
        // Reuse create_task and delete_task which are now routed!
        let mut new_task = task.clone();
        new_task.calendar_href = new_calendar_href.to_string();
        new_task.href = String::new();
        new_task.etag = String::new();

        self.create_task(&mut new_task).await?;

        if let Err(e) = self.delete_task(task).await {
            eprintln!("Warning: delete failed during move: {}", e);
        }
        Ok(new_task)
    }

    // In src/client.rs inside impl RustyClient

    pub async fn migrate_tasks(
        &self,
        tasks: Vec<Task>,
        target_calendar_href: &str,
    ) -> Result<usize, String> {
        let mut success_count = 0;
        for task in tasks {
            // Reuses the robust move_task (Create -> Delete) logic
            if self.move_task(&task, target_calendar_href).await.is_ok() {
                success_count += 1;
            }
        }
        Ok(success_count)
    }
    pub async fn sync_journal(&self) -> Result<(), String> {
        let mut journal = Journal::load();
        if journal.is_empty() {
            return Ok(());
        }

        if let Some(client) = &self.client {
            // Process queue
            while !journal.is_empty() {
                // Get a mutable reference to the first action
                let action = &mut journal.queue[0];
                
                let mut should_pop = false;
                let mut fatal_error = None;

                match action {
                    Action::Create(task) => {
                        let filename = format!("{}.ics", task.uid);
                        let full_href = if task.calendar_href.ends_with('/') {
                            format!("{}{}", task.calendar_href, filename)
                        } else {
                            format!("{}/{}", task.calendar_href, filename)
                        };
                        let bytes = task.to_ics().as_bytes().to_vec();
                        
                        match client.create_resource(&full_href, bytes, b"text/calendar").await {
                            Ok(_) => should_pop = true,
                            Err(e) => {
                                let err_s = format!("{:?}", e);
                                if err_s.contains("412") || err_s.contains("PreconditionFailed") {
                                    should_pop = true; 
                                } else {
                                    fatal_error = Some(err_s);
                                }
                            }
                        }
                    }
                    Action::Update(task) => {
                        let bytes = task.to_ics().as_bytes().to_vec();
                        match client.update_resource(
                                &task.href,
                                bytes.clone(),
                                &task.etag,
                                b"text/calendar; charset=utf-8; component=VTODO",
                            ).await 
                        {
                            Ok(_) => should_pop = true,
                            Err(e) => {
                                let err_s = format!("{:?}", e);
                                if err_s.contains("412") || err_s.contains("PreconditionFailed") {
                                    println!("412 Conflict on Update. Fetching fresh ETag...");
                                    
                                    if let Ok(fresh_vec) = client.get_calendar_resources(&task.calendar_href, &[task.href.clone()]).await 
                                       && let Some(fresh_item) = fresh_vec.first() 
                                    {
                                        // CORRECTED: Check inside the content Result
                                        if let Ok(content) = &fresh_item.content {
                                            println!("Fresh ETag found: {}. Retrying...", content.etag);
                                            
                                            // 2. Update local ETag
                                            task.etag = content.etag.clone();
                                            
                                            // 3. Retry immediately with Last-Write-Wins (our bytes, new ETag)
                                            let _ = client.update_resource(
                                                &task.href,
                                                bytes, 
                                                &task.etag, 
                                                b"text/calendar; charset=utf-8; component=VTODO"
                                            ).await;
                                            
                                            should_pop = true;
                                        } else {
                                            // Resource likely deleted on server (content is Err)
                                            should_pop = true;
                                        }
                                    } else {
                                        // Could not fetch (404?), so it's gone.
                                        should_pop = true;
                                    }
                                } else if err_s.contains("404") {
                                    should_pop = true;
                                } else {
                                    fatal_error = Some(err_s);
                                }
                            }
                        }
                    }
                    Action::Delete(task) => {
                        match client.delete(&task.href, &task.etag).await {
                            Ok(_) => should_pop = true,
                            Err(e) => {
                                let err_s = format!("{:?}", e);
                                if err_s.contains("404") {
                                    should_pop = true;
                                } else if err_s.contains("412") || err_s.contains("PreconditionFailed") {
                                     if let Ok(fresh_vec) = client.get_calendar_resources(&task.calendar_href, &[task.href.clone()]).await 
                                       && let Some(fresh_item) = fresh_vec.first() 
                                    {
                                        // CORRECTED: Check inside the content Result
                                        if let Ok(content) = &fresh_item.content {
                                            // Retry Delete with new ETag
                                            let _ = client.delete(&task.href, &content.etag).await;
                                        }
                                        should_pop = true;
                                    } else {
                                        should_pop = true;
                                    }
                                } else {
                                    fatal_error = Some(err_s);
                                }
                            }
                        }
                    }
                }

                if should_pop {
                    let _ = journal.pop_front(); 
                } else {
                    eprintln!("Journal Sync Paused: {}", fatal_error.unwrap_or_default());
                    break;
                }
            }
        }
        Ok(())
    }
}

// ... NoVerifier struct ...
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
