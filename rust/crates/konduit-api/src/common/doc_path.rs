pub struct DocPath(String);

impl DocPath {
    pub fn to_git_cmd(&self, vcs_hash: &str) -> String {
        format!("git show {}:{}", vcs_hash, self.0)
    }
    pub fn to_web_url(&self, base: &str) -> String {
        let path = self.0.trim_start_matches("docs/").trim_end_matches(".md");
        format!("{}/{}", base.trim_end_matches('/'), path)
    }
    pub fn to_local_path(&self) -> String {
        let path = self.0.trim_start_matches("docs/").trim_end_matches(".md");
        format!("docs/book/{}.html", path)
    }
}
