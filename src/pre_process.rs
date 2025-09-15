use std::sync::Arc;

use reqwest::{ClientBuilder, header};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use scraper;
use serde_json::Value;

pub async fn get_scan_id(
    url: &String,
    token: &String,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let get_book_resource_url =
        "https://ereserves.lib.tsinghua.edu.cn/userapi/ReadBook/GetResourcesUrl";
    let get_book_read_id_url = format!(
        "https://ereserves.lib.tsinghua.edu.cn/userapi/MyBook/getBookDetail?bookId={}",
        url.rsplit('/').nth(0).unwrap()
    );
    let mut default_headers = header::HeaderMap::new();
    default_headers.insert("Jcclient", header::HeaderValue::from_str(token.as_str())?);
    default_headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.71 Safari/537.36"));
    let client = ClientBuilder::new()
        .default_headers(default_headers)
        .build()?;
    let res = client
        .get(get_book_read_id_url)
        .send()
        .await?
        .text()
        .await?;
    let v: Value = serde_json::from_str(&res)?;
    let book_real_id = v["data"]["jc_ebook_vo"]["urls"][0]["READURL"]
        .as_str()
        .unwrap()
        .to_string();
    let res = client
        .post(get_book_resource_url)
        .json(&serde_json::json!({"id":book_real_id}))
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
    let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
    let client = ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_provider(Arc::clone(&cookie_store))
        .build()?;
    let res = client.get(book_access_url).send().await?;
    let botu_read_kernel = res
        .cookies()
        .find(|cookie| cookie.name() == "BotuReadKernel")
        .unwrap()
        .value()
        .to_string();
    let res = ClientBuilder::new()
        .cookie_provider(Arc::clone(&cookie_store))
        .build()?
        .get(res.headers().get("Location").unwrap().to_str()?)
        .send()
        .await?;
    let doc = scraper::Html::parse_document(res.text().await?.as_str());
    let selector = scraper::Selector::parse("#scanid").unwrap();
    let scan_id = doc
        .select(&selector)
        .nth(0)
        .unwrap()
        .value()
        .attr("value")
        .unwrap()
        .to_string();
    Ok((botu_read_kernel, book_real_id, scan_id))
}
