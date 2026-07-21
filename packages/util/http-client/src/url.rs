pub fn clean_base(base_url: &str) -> &str {
    base_url.strip_suffix('/').unwrap_or(base_url)
}

pub fn clean_path(path: &str) -> &str {
    path.strip_prefix('/').unwrap_or(path)
}

/// Both already clean!
pub fn join(base_url: &str, path: &str) -> String {
    let mut url = String::with_capacity(base_url.len() + 1 + path.len());
    url.push_str(base_url);
    url.push('/');
    url.push_str(path);
    url
}

/// Base already clean, path to be cleaned
pub fn clean_join(base_url: &str, path: &str) -> String {
    join(base_url, clean_path(path))
}

/// Base already clean, path to be cleaned
pub fn clean_both(base_url: &str, path: &str) -> String {
    join(clean_base(base_url), clean_path(path))
}
