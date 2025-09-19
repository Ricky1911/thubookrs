use image::ImageReader;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use std::io::Cursor;

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

fn img2pdf(
    imgs: Vec<PathBuf>,
    pdf_path: &Path,
    quality: i32,
    auto_resize: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let intermediate_dir = pdf_path.parent().unwrap().join("intermediate");
    if !intermediate_dir.exists() {
        fs::create_dir_all(intermediate_dir)?;
    }
    let mut doc = Document::with_version("2.0");
    let pages_id = doc.new_object_id();
    let mut page_objects = Vec::new();
    let mut width = 0;
    let mut height = 0;
    for img in imgs {
        let img = ImageReader::open(img)?.decode()?;
        width = img.width() as i32;
        height = img.height() as i32;
        let mut img_bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut img_bytes), image::ImageFormat::Jpeg)?;
        let image_xobject = Stream::new(
            lopdf::Dictionary::from_iter(vec![
                ("Type", "XObject".into()),
                ("Subtype", "Image".into()),
                ("Width", img.width().into()),
                ("Height", img.height().into()),
                ("ColorSpace", "DeviceRGB".into()),
                ("BitsPerComponent", 8.into()),
                ("Filter", "DCTDecode".into()),
            ]),
            img_bytes,
        );
        let content = Content { operations: vec![] };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });

        doc.insert_image(
            page_id,
            image_xobject,
            (0 as f32, 0 as f32),
            (img.width() as f32, img.height() as f32),
        )?;
        page_objects.push(page_id.into());
        println!("{}", page_objects.len())
    }

    let count = page_objects.len();
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_objects,
        "Count" => count as i32,
        "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
    };

    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);
    doc.compress();
    let mut file = std::fs::File::create(pdf_path)?;
    doc.save_modern(&mut file)?;
    Ok(())
}

mod test {
    use std::{path::PathBuf, str::FromStr};

    use crate::convert::{get_images, img2pdf};

    #[test]
    fn basic_test() {
        let imgs = get_images(
            &PathBuf::from_str(
                "D:\\works\\projects\\thubookrs\\downloads\\5e9eb2048a4d4605be86465ad685b1c8",
            )
            .unwrap(),
        );

        let result = img2pdf(
            imgs.into_iter().take(1oo).collect(),
            &PathBuf::from_str("test.pdf").unwrap(),
            10,
            true,
        );
        if let Err(e) = result {
            println!("{}", e);
        }
    }
}
