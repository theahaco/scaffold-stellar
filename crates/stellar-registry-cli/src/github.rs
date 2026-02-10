pub struct Fetcher<'a> {
    github: &'a str,
    package: &'a str,
    bin_ver: &'a str,
}
impl<'a> Fetcher<'a> {
    pub fn new<T: AsRef<str>>(github: &'a T, package: &'a T, bin_ver: &'a T) -> Self {
        Self {
            github: github.as_ref(),
            package: package.as_ref(),
            bin_ver: bin_ver.as_ref(),
        }
    }

    pub async fn fetch(&self) -> Result<Vec<u8>, reqwest::Error> {
        let Fetcher {
            github,
            package,
            bin_ver,
        } = self;
        let url = format!(
            "https://github.com/{github}/releases/download/{package}-v{bin_ver}/{package}_v{bin_ver}.wasm"
        );
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "stellar-registry-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }
        Ok(response.bytes().await?.to_vec())
    }
}
