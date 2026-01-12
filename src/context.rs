use axum::http::HeaderMap;

/// Common context data available to all page templates
pub struct PageContext {
    pub is_home: bool,
    pub client_ip: String,
    pub dnt_enabled: bool,
    pub page_hits: u64,
    pub unique_hits: u64,
}

impl PageContext {
    /// Create a new PageContext from request headers
    /// For non-home pages
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            is_home: false,
            client_ip: extract_client_ip(headers),
            dnt_enabled: headers
                .get("dnt")
                .map(|v| v.to_str().unwrap_or("0") == "1")
                .unwrap_or(false),
            // TODO: Implement PHPCount database integration
            page_hits: 0,
            unique_hits: 0,
        }
    }

    /// Create context for the home page (no footer)
    pub fn home_page(headers: &HeaderMap) -> Self {
        Self {
            is_home: true,
            client_ip: extract_client_ip(headers),
            dnt_enabled: headers
                .get("dnt")
                .map(|v| v.to_str().unwrap_or("0") == "1")
                .unwrap_or(false),
            page_hits: 0,
            unique_hits: 0,
        }
    }
}

fn extract_client_ip(headers: &HeaderMap) -> String {
    // Check X-Forwarded-For first (for reverse proxy setups)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            // Take the first IP if there are multiple
            return s.split(',').next().unwrap_or(s).trim().to_string();
        }
    }

    // Check X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            return s.to_string();
        }
    }

    // Fallback - in production this would come from the connection info
    // For now, return a placeholder
    "127.0.0.1".to_string()
}
