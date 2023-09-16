use redb::ReadableTable;

use crate::{
    config::Extension,
    data_struct,
    utils::{CACHER, TABLE_HTTP_CLIENT},
};

use super::Query;

#[derive(Debug, Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().gzip(true).build()?;
        Ok(Self { client })
    }

    pub async fn get_extension_response(
        &self,
        extensions: &[Extension],
    ) -> anyhow::Result<data_struct::IRawGalleryQueryResult> {
        let query = Query::new(extensions);
        let body = serde_json::to_string(&query)?;
        Ok(self
            .client
            .post("https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery")
            .header(
                "Accept",
                "Application/json; charset=utf-8; api-version=7.2-preview.1",
            )
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?
            .json::<data_struct::IRawGalleryQueryResult>()
            .await?)
    }

    pub async fn request_get_remote_object<T: for<'de> serde::Deserialize<'de>>(
        &self,
        url: &str,
    ) -> anyhow::Result<T> {
        let value = (|| -> anyhow::Result<String> {
            let r_txn = CACHER.begin_read()?;
            let table = r_txn.open_table(TABLE_HTTP_CLIENT)?;
            let value = table
                .get(url)?
                .ok_or_else(|| redb::Error::InvalidSavepoint)?
                .value()
                .to_string();

            Ok(value)
        })();

        if let Ok(value) = value {
            return Ok(serde_json::from_str(&value).unwrap());
        }

        let req = self.client.get(url).build().unwrap();
        let rep = self
            .client
            .execute(req)
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let _ = (|| -> anyhow::Result<()> {
            let wt = CACHER.begin_write()?;
            {
                let mut table = wt.open_table(TABLE_HTTP_CLIENT)?;
                table.insert(url, rep.as_str())?;
            }
            wt.commit()?;

            Ok(())
        })();

        Ok(serde_json::from_str(&rep).unwrap())
    }
}