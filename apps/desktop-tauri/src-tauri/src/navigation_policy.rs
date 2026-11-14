pub fn is_local_application_url(url: &tauri::Url, development: bool) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match (url.scheme(), url.host_str(), url.port()) {
        ("tauri", Some("localhost"), None) => true,
        ("http", Some("tauri.localhost"), None) => true,
        ("http", Some("127.0.0.1"), Some(1420)) => development,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_local_application_url;

    #[test]
    fn only_exact_application_origins_are_navigable() {
        for candidate in ["tauri://localhost/index.html", "http://tauri.localhost/"] {
            assert!(is_local_application_url(
                &tauri::Url::parse(candidate).unwrap(),
                false
            ));
        }
        for candidate in [
            "https://example.com/",
            "http://tauri.localhost.example.com/",
            "http://user@tauri.localhost/",
            "http://127.0.0.1:1421/",
            "file:///C:/private.txt",
            "javascript:alert(1)",
        ] {
            assert!(!is_local_application_url(
                &tauri::Url::parse(candidate).unwrap(),
                true
            ));
        }
        let dev_url = tauri::Url::parse("http://127.0.0.1:1420/").unwrap();
        assert!(is_local_application_url(&dev_url, true));
        assert!(!is_local_application_url(&dev_url, false));
    }
}
