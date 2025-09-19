use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use rand::Rng;
use reqwest::{Client, ClientBuilder, header};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};

use crate::pre_process::DownloadTask;

fn get_tmp_name() -> PathBuf {
    let rand_string: String = rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    PathBuf::from(format!(".tmp{}", rand_string))
}

#[derive(Clone)]
pub struct Downloader {
    client: Client,
    //cookie_store: Arc<CookieStoreMutex>,
}

impl Downloader {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut default_headers = header::HeaderMap::new();
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        default_headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.71 Safari/537.36"));
        Ok(Self {
            client: ClientBuilder::new()
                .default_headers(default_headers)
                .cookie_provider(Arc::clone(&cookie_store))
                .build()?,
            //cookie_store,
        })
    }

    async fn download_one(
        &self,
        botu_read_kernel: &str,
        img_path: &str,
        save_dir: &Path,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = "https://ereserves.lib.tsinghua.edu.cn/readkernel/JPGFile/DownJPGJsNetPage";
        let save_path = save_dir.join(filename);
        let res = self
            .client
            .get(url)
            .query(&[("filePath", img_path)])
            .header("Cookie", format!("BotuReadKernel={}", botu_read_kernel))
            .send();
        println!("Start Downloading: {}", &filename);
        let tmp_name = loop {
            let tmp_name = get_tmp_name();
            if !tmp_name.exists() {
                break tmp_name;
            }
        };
        let bytes = res.await?.bytes().await?;
        let mut file = fs::File::create(&tmp_name)?;
        file.write_all(&bytes)?;
        fs::rename(tmp_name, save_path)?;
        println!("Download success: {}", filename);
        Ok(())
    }

    pub async fn download_imgs(
        &self,
        task: DownloadTask,
        save_dir: &Path,
        thread_num: usize,
        cancel: tokio_util::sync::CancellationToken,
    ) -> bool {
        if !save_dir.exists() {
            fs::create_dir_all(save_dir).unwrap();
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(thread_num)
            .enable_all()
            .build()
            .unwrap();
        let mut download_names = Vec::new();
        let mut handles = Vec::new();
        for (chap_num, img_urls) in task.page_urls.iter().enumerate() {
            for (page_num, img_path) in img_urls.iter().enumerate() {
                let filename = format!(
                    "{}_{}.{}",
                    chap_num,
                    page_num,
                    img_path
                        .split('/')
                        .next_back()
                        .unwrap()
                        .split('.')
                        .next_back()
                        .unwrap()
                );
                let path = save_dir.join(&filename);
                if path.exists() {
                    println!("Already downloaded: {}, skip", &filename);
                    continue;
                }
                download_names.push(filename.clone());
                let botu_read_kernel = task.botu_read_kernel.clone();
                let img_path = img_path.clone();
                let save_dir = save_dir.to_path_buf();
                let self_clone = self.clone();
                let cancel = cancel.clone();
                let handle = runtime.spawn(async move {
                    tokio::select! {
                        result = self_clone
                        .download_one(&botu_read_kernel, &img_path, &save_dir, &filename)
                        => { result }
                        _ = cancel.cancelled() => { Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Keyboard interrupted").into()) }
                    }
                });
                handles.push(handle);
            }
        }

        let mut success = true;
        for handle in handles {
            let result = handle.await;
            if let Ok(result) = result
                && let Err(e) = result {
                    println!("{}", e);
                    success = false;
                }
        }
        runtime.shutdown_background();
        success
    }
}
