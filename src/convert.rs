use image::ImageReader;
use lopdf::content::Content;
use lopdf::{Document, Object, SaveOptions, Stream, dictionary};

use std::collections::HashMap;
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
};

fn get_images(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for entry in fs::read_dir(dir).expect(&format!("Can't open directory {}", dir.display())) {
        if let Ok(entry) = entry {
            if let Ok(filetype) = entry.file_type()
                && filetype.is_file()
            {
                result.push(entry.file_name());
            }
        }
    }
    result.sort_by(|a, b| {
        let a = a.to_str().unwrap();
        let b = b.to_str().unwrap();
        let mut iter_a = a.split('_');
        let mut iter_b = b.split('_');
        let a_chap: i32 = iter_a.next().unwrap().parse().unwrap();
        let b_chap: i32 = iter_b.next().unwrap().parse().unwrap();
        let order = a_chap.cmp(&b_chap);
        match order {
            Ordering::Equal => {
                let a_page: i32 = iter_a
                    .next()
                    .unwrap()
                    .split('.')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                let b_page: i32 = iter_b
                    .next()
                    .unwrap()
                    .split('.')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                a_page.cmp(&b_page)
            }
            _ => order,
        }
    });
    result.iter().map(|filename| dir.join(filename)).collect()
}

async fn pre_process_imgs(
    imgs: &Vec<PathBuf>,
    intermediate_dir: &Path,
    quality: i32,
    auto_resize: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let total = imgs.len();
    let mut handles: Vec<
        tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    > = Vec::with_capacity(total);
    let mut common_size = None;
    if auto_resize {
        let mut size_count = HashMap::new();
        for img_path in imgs {
            let img = ImageReader::open(&img_path)?.decode()?;
            let width = img.width();
            let height = img.height();
            match size_count.get_mut(&(width, height)) {
                None => {
                    size_count.insert((width, height), 1);
                }
                Some(count) => {
                    *count += 1;
                }
            }
        }
        common_size = size_count
            .drain()
            .max_by_key(|(_, count)| *count)
            .map(|(size, _)| size)
            .take();
    }
    if let Some((width, height)) = common_size {
        println!("Auto resizing with width: {width}, height: {height}");
    }
    for img_path in imgs {
        let img_path_clone = img_path.clone();
        let intermediate_dir_clone = intermediate_dir.to_path_buf();
        let handle = runtime.spawn_blocking(move || {
            let file_name = img_path_clone.file_name().unwrap();
            let output_path = intermediate_dir_clone.join(file_name);
            if output_path.exists() {
                println!("Resize already completed: {}, skip", file_name.display());
                return Ok(());
            }
            let img = ImageReader::open(&img_path_clone)?.decode()?;
            let (width, height) = if let Some((width, height)) = common_size {
                (width, height)
            } else {
                (img.width(), img.height())
            };

            let img = img.resize(
                width / 10 * quality as u32,
                height / 10 * quality as u32,
                image::imageops::FilterType::Lanczos3,
            );

            let output_path = intermediate_dir_clone.join(file_name);
            img.save(&output_path)?;
            println!("Resize complete: {}", file_name.display());
            Ok(())
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.await;
    }
    runtime.shutdown_background();
    Ok(())
}

async fn img2pdf(
    imgs: Vec<PathBuf>,
    pdf_path: &Path,
    quality: i32,
    auto_resize: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let intermediate_dir = pdf_path.parent().unwrap().join("intermediate");
    if !intermediate_dir.exists() {
        fs::create_dir_all(&intermediate_dir)?;
    }
    if let Err(e) = pre_process_imgs(&imgs, &intermediate_dir, quality, auto_resize).await {
        println!("Convert failed: {}", e);
    }

    let mut doc = Document::with_version("2.0");
    let pages_id = doc.new_object_id();
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    let total = imgs.len();
    let mut page_objects = Vec::with_capacity(total);
    for (index, img_path) in imgs.iter().enumerate() {
        let img_path = intermediate_dir.join(img_path.file_name().unwrap());
        let image_xobject = lopdf::xobject::image(&img_path)?;
        let content = Content { operations: vec![] };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let width = image_xobject.dict.get(b"Width").unwrap().as_i64().unwrap();
        let height = image_xobject.dict.get(b"Height").unwrap().as_i64().unwrap();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        });
        doc.insert_image(
            page_id,
            image_xobject,
            (0 as f32, 0 as f32),
            (width as f32, height as f32),
        )?;
        page_objects.push(page_id.into());
        println!("Convert complete: {}/{total}", index + 1)
    }

    let count = page_objects.len();
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_objects,
        "Count" => count as i32,
    };

    doc.objects.insert(pages_id, Object::Dictionary(pages));

    fs::remove_dir_all(&intermediate_dir)?;
    doc.compress();
    let mut file = std::fs::File::create(pdf_path)?;
    doc.save_with_options(
        &mut file,
        SaveOptions::builder()
            .linearize(true)
            .use_object_streams(true)
            .use_xref_streams(true)
            .compression_level(9)
            .max_objects_per_stream(count + 2)
            .build(),
    )?;
    //doc.save_modern(&mut file)?;
    Ok(())
}

pub async fn convert(
    dir: &Path,
    pdf_path: &Path,
    quality: i32,
    auto_resize: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let imgs = get_images(dir);
    img2pdf(imgs, pdf_path, quality, auto_resize).await?;
    Ok(())
}
