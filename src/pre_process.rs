use std::{collections::HashMap, sync::Arc};

use reqwest::{Client, ClientBuilder, header};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use scraper::{Html, Selector};
use serde_json::Value;

pub struct DownloadTask {
    pub book_real_id: String,
    pub botu_read_kernel: String,
    pub page_urls: Vec<Vec<String>>,
}

pub struct Preprocessor {
    client: Client,
    client_no_redirect: Client,
    cookie_store: Arc<CookieStoreMutex>,
}

impl Preprocessor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        let mut default_headers = header::HeaderMap::new();
        // default_headers.insert("Jcclient", header::HeaderValue::from_str(token.as_str())?);
        default_headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.71 Safari/537.36"));
        Ok(Self {
            client: ClientBuilder::new()
                .cookie_provider(Arc::clone(&cookie_store))
                .default_headers(default_headers.clone())
                .build()?,
            client_no_redirect: ClientBuilder::new()
                .redirect(reqwest::redirect::Policy::none())
                .cookie_provider(Arc::clone(&cookie_store))
                .default_headers(default_headers)
                .build()?,
            cookie_store,
        })
    }

    async fn get_scan_id(
        &self,
        url: &str,
        token: &str,
    ) -> Result<(String, String, String), Box<dyn std::error::Error>> {
        let get_book_resource_url =
            "https://ereserves.lib.tsinghua.edu.cn/userapi/ReadBook/GetResourcesUrl";
        let get_book_read_id_url = format!(
            "https://ereserves.lib.tsinghua.edu.cn/userapi/MyBook/getBookDetail?bookId={}",
            url.rsplit('/').nth(0).unwrap()
        );

        let res = self
            .client
            .get(get_book_read_id_url)
            .header("Jcclient", token)
            .send()
            .await?
            .text()
            .await?;
        let v: Value = serde_json::from_str(&res)?;
        let book_real_id = v["data"]["jc_ebook_vo"]["urls"][0]["READURL"]
            .as_str()
            .unwrap()
            .to_owned();

        let res = self
            .client
            .post(get_book_resource_url)
            .json(&serde_json::json!({"id":book_real_id}))
            .header("Jcclient", token)
            .send()
            .await?
            .text()
            .await?;
        let v: Value = serde_json::from_str(&res)?;
        if v["info"].as_str().unwrap() != "成功" {
            println!("{}", v);
            panic!("Token error, please retry");
        }
        let book_access_url = v["data"].as_str().unwrap();

        let res = self.client_no_redirect.get(book_access_url).send().await?;
        let botu_read_kernel = res
            .cookies()
            .find(|cookie| cookie.name() == "BotuReadKernel")
            .unwrap()
            .value()
            .to_owned();

        let res = self
            .client
            .get(res.headers().get("Location").unwrap().to_str()?)
            .send()
            .await?;
        let doc = Html::parse_document(res.text().await?.as_str());
        let selector = Selector::parse("#scanid").unwrap();
        let scan_id = doc
            .select(&selector)
            .nth(0)
            .unwrap()
            .value()
            .attr("value")
            .unwrap()
            .to_owned();

        Ok((botu_read_kernel, book_real_id, scan_id))
    }

    async fn get_book_chapters(
        &self,
        botu_read_kernel: &str,
        scan_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = "https://ereserves.lib.tsinghua.edu.cn/readkernel/KernelAPI/BookInfo/selectJgpBookChapters";
        let mut form = HashMap::new();
        form.insert("SCANID", scan_id);
        let res = self
            .client
            .post(url)
            .header("BotuReadKernel", botu_read_kernel)
            .form(&form)
            .send()
            .await?
            .text()
            .await?;
        let v: Value = serde_json::from_str(res.as_str())?;
        let info_array = v["data"].as_array().unwrap();
        let emids: Vec<String> = info_array
            .iter()
            .map(|info| info["EMID"].as_str().unwrap().to_owned())
            .collect();
        Ok(emids)
    }

    async fn get_book_pages(
        &self,
        botu_read_kernel: &str,
        book_real_id: &str,
        emids: Vec<String>,
    ) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
        let url = "https://ereserves.lib.tsinghua.edu.cn/readkernel/KernelAPI/BookInfo/selectJgpBookChapter";
        let mut page_urls = Vec::new();
        for emid in emids {
            let mut form = HashMap::new();
            form.insert("EMID", emid);
            form.insert("BOOKID", book_real_id.to_owned());
            let res = self
                .client
                .post(url)
                .header("BotuReadKernel", botu_read_kernel)
                .form(&form)
                .send()
                .await?
                .text()
                .await?;
            let v: Value = serde_json::from_str(res.as_str()).unwrap();
            let info_array = v["data"]["JGPS"].as_array().unwrap();
            page_urls.push(
                info_array
                    .iter()
                    .map(|info| info["hfsKey"].as_str().unwrap().to_owned())
                    .collect(),
            );
        }

        Ok(page_urls)
    }

    pub async fn parse(
        &self,
        url: &str,
        token: &str,
    ) -> Result<DownloadTask, Box<dyn std::error::Error>> {
        let (botu_read_kernel, book_real_id, scan_id) = self.get_scan_id(url, token).await?;
        let emids = self.get_book_chapters(&botu_read_kernel, &scan_id).await?;
        let page_urls = self
            .get_book_pages(&botu_read_kernel, &book_real_id, emids)
            .await?;
        self.cookie_store.lock().unwrap().clear();
        Ok(DownloadTask {
            book_real_id,
            botu_read_kernel,
            page_urls,
        })
    }
}
